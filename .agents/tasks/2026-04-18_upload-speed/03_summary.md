# upload-speed — Summary

## Change

Replaced `client.client.upload_file(local_path)` with `upload_stream` in
`src/upload/mod.rs` (`upload_file` function, lines 105–122).

## Files modified

- `src/upload/mod.rs` — 5 lines → 18 lines at the upload call site
- `Cargo.toml` — **unchanged** (`tokio-util` not required)

## Correction from request sketch

The request proposed a `tokio_util::io::ReaderStream` adapter. The actual
`upload_stream` signature is `(&self, stream: &mut S: AsyncRead + Unpin, size: usize, name: String)`.
`tokio::fs::File` satisfies `AsyncRead + Unpin` directly — no adapter needed.

## Why this improves throughput

- `upload_file` uploads parts sequentially.
- `upload_stream` for files > 10 MB (`BIG_FILE_SIZE`) spawns 4 concurrent
  worker tasks (`WORKER_COUNT = 4`) that pull 512 KB parts from a shared
  `Arc<PartStream>` — this is the real throughput gain.
- Chunking (512 KB = `MAX_CHUNK_SIZE`) is grammers-internal and unchanged.

## Verification

`cargo fmt -- --check` ✓  
`cargo clippy -- -D warnings` ✓  
`cargo build --release` ✓  
`cargo test -- --test-threads=1` ✓ (2 passed, 1 ignored)
