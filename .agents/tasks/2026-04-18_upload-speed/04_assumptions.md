# upload-speed — Assumptions & Shortcuts

Task: `2026-04-18_upload-speed`
Scope: Replace `upload_file` with `upload_stream` in `src/upload/mod.rs`

---

## 1. `tokio-util` not needed (request sketch was wrong)

**Request stated:** Add `tokio-util` to `Cargo.toml`; wrap the file in a
`ReaderStream` and adapt it with `.map(...)`.

**What I did:** Verified the actual `upload_stream` signature before writing
any code. The API is `(&self, stream: &mut S: AsyncRead + Unpin, size: usize, name: String)` — it takes an `AsyncRead`, not a `Stream<Item = Result<Bytes>>`.
`tokio::fs::File` satisfies `AsyncRead + Unpin` directly; no adapter is needed.

**Assumption:** The request's sketch was a reference approximation, not a
binding spec. The instruction "verify the exact signature and adapt" took
precedence. No `Cargo.toml` change was made.

---

## 2. `unwrap_or_default()` on `file_name()`

**Request stated:** Use `unwrap_or_default()` for the file name.

**What I did:** Kept `unwrap_or_default()`, which yields `""` for malformed
paths. grammers internally replaces an empty name with `"a"` (verified in
source). Functionally identical to what `upload_file` produced (it uses
`unwrap()` with a comment that `..` paths are impossible for uploadable files).

**Assumption:** An empty-string fallback is acceptable; the grammers safety
net handles it. No explicit panic or error was introduced.

---

## 3. `file.metadata()` called on the open handle (not a second `stat`)

**What I did:** Called `.metadata()` on the already-open `tokio::fs::File`
handle rather than a separate `tokio::fs::metadata(local_path)` call.

**Rationale:** One syscall instead of two; avoids a TOCTOU window between
`stat` and `open`. This was not specified in the request.

**Assumption:** File size does not change between `open` and `metadata()` on
the same handle (always true for the local files this tool uploads).

---

## 4. `u64` → `usize` cast for `file_size`

**What I did:** `file.metadata().await?.len() as usize`

**Request stated:** `file_size = ... .len() as usize` — identical.

**Hidden assumption:** The host is a 64-bit system where `usize` is 8 bytes.
On a 32-bit target, files > 4 GiB would silently truncate. Accepted because:
- The existing codebase already compares file sizes to `FOUR_GIB: u64 = 4 * 1024 * 1024 * 1024`
  and the project targets a single amd64 Linux host.
- Telegram's upload limit is 2 GiB (4 GiB for premium), well within `usize`
  on any 64-bit system.

---

## 5. No `retry_flood_wait!` wrapper on `upload_stream`

**Architecture rule:** "Every call site that touches the Telegram API must be
wrapped in `retry_flood_wait!`."

**What I did:** Did not add `retry_flood_wait!` around `upload_stream`.

**Rationale / shortcut:** `upload_stream` returns `std::io::Error`, not a
grammers `InvocationError` that can carry a `FloodWait` payload.
`retry_flood_wait!` pattern-matches on `InvocationError::Rpc` — wrapping an
`io::Error` branch would be a type error. The original `upload_file` was also
unwrapped for the same reason. Upload operations go through a different
MTProto path (`upload.saveBigFilePart` / `upload.saveFilePart`) that does not
trigger user-facing flood-wait in the same way as message-send operations.

**Risk:** If Telegram does return a flood-wait on an upload part, grammers
surfaces it as an `io::Error` and we propagate it as `UploadWarning`, which
is the same behaviour as before this change.

---

## 6. No new `use` statement added

**What I did:** Used `tokio::fs::File::open(...)` fully-qualified rather than
adding `use tokio::fs::File;`.

**Rationale:** `use tokio::fs;` is already present at line 13. A fully-qualified
path avoids a new import line and keeps the diff minimal. `cargo clippy` did
not flag it.
