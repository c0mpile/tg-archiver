# upload-speed — Replace `upload_file` with `upload_stream`

## Summary

Replace the grammers convenience wrapper `upload_file` with the lower-level
`upload_stream` call in `src/upload/mod.rs`. The goal is to avoid
`upload_file`'s internal seek-and-reopen overhead and allow grammers to drive
its own concurrent worker pool against a file opened by us.

---

## Research Findings

### 1. `tokio-util` dependency check

`tokio-util` is **not present** in `Cargo.toml`. However, after verifying the
actual `upload_stream` signature (see below), **`tokio-util` is not needed**
for this change. The API takes `AsyncRead + Unpin`, which `tokio::fs::File`
satisfies directly — no `ReaderStream` adapter is required.

### 2. Verified `upload_stream` signature

Source:
```
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/grammers-client-0.9.0/src/client/files.rs
```

```rust
pub async fn upload_stream<S: AsyncRead + Unpin>(
    &self,
    stream: &mut S,
    size: usize,
    name: String,
) -> Result<Uploaded, io::Error>
```

Key internals:
- **Chunking is internal to grammers.** `PartStream` always reads `MAX_CHUNK_SIZE`
  (512 KB) bytes per part. There is no caller-controlled chunk size on
  `upload_stream` — grammers owns that detail.
- For files > 10 MB (`BIG_FILE_SIZE`), grammers spawns **4 parallel worker tasks**
  (`WORKER_COUNT = 4`) that each pull the next part from a shared `Arc<PartStream>`.
  This is the actual source of throughput improvement over `upload_file`, which
  appears to upload serially.
- The `name: String` parameter is passed by value (not `&str`).

> **Correction to the request sketch:** The request proposed wrapping the file
> in a `tokio_util::io::ReaderStream` and calling a stream-of-`Bytes` API.
> The real `upload_stream` takes `&mut S: AsyncRead + Unpin` — a plain
> `tokio::fs::File` is the correct argument. No `tokio-util` is needed.

### 3. `upload_file` existing call

File: `src/upload/mod.rs`, lines 105–109:

```rust
let uploaded = client
    .client
    .upload_file(local_path)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))?;
```

`upload_file` internally opens the file, seeks to measure its size, then
rewinds and uploads serially in 512 KB parts (no concurrency).

---

## Proposed Change

### `src/upload/mod.rs` — only file modified

Replace lines 105–109 with:

```rust
let file_name = local_path
    .file_name()
    .unwrap_or_default()
    .to_string_lossy()
    .into_owned();
let mut file = tokio::fs::File::open(local_path)
    .await
    .context("Failed to open file for upload")?;
let file_size = file
    .metadata()
    .await
    .context("Failed to read file metadata")?
    .len() as usize;
let uploaded = client
    .client
    .upload_stream(&mut file, file_size, file_name)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))?;
```

No other logic, retry behaviour, or flood-wait handling is touched.

### `Cargo.toml` — **no change required**

`tokio-util` is not needed; `tokio::fs::File` implements `AsyncRead + Unpin`
natively.

---

## Exact Diff

```diff
--- a/src/upload/mod.rs
+++ b/src/upload/mod.rs
@@ -105,7 +105,16 @@
-    let uploaded = client
-        .client
-        .upload_file(local_path)
-        .await
-        .map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))?;
+    let file_name = local_path
+        .file_name()
+        .unwrap_or_default()
+        .to_string_lossy()
+        .into_owned();
+    let mut file = tokio::fs::File::open(local_path)
+        .await
+        .context("Failed to open file for upload")?;
+    let file_size = file
+        .metadata()
+        .await
+        .context("Failed to read file metadata")?
+        .len() as usize;
+    let uploaded = client
+        .client
+        .upload_stream(&mut file, file_size, file_name)
+        .await
+        .map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))?;
```

---

## Open Questions

None. The signature is unambiguous and the change is fully self-contained.

---

## Verification Plan

```
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --release
cargo test -- --test-threads=1
```
