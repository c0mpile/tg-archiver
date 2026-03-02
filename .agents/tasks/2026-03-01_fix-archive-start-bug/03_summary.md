# Fix Archive Start Bug (Upload Issue) - Summary

## Root Cause
1. **Peer Cache Miss:** When resuming or starting an archive run, `AppEvent::StartArchiveRun` checked *only* if the source channel was in `peer_cache`. If it was, it completely bypassed loading joined groups. As a result, the destination group peer was never loaded, causing the upload step to silently fail with "Destination peer not found in cache".
2. **Unconditional Download & Truncation:** The `tokio::fs::File::create` block and subsequent loop were outside the `needs_download` logic entirely. For previously completed files resuming an upload attempt, the file was truncated to 0 bytes.
3. **Open File Descriptor:** A race condition existed where the `tokio::fs::File` descriptor was not dropped before `upload_media` was invoked, meaning `upload_file` read the file while it was still being locked/managed by the OS flush tables, often leading to sizing inconsistencies.
4. **State Machine Sync:** We also verified that `AppEvent::DownloadProgress` is invoked properly following success, maintaining consistency with atomic database writes.

## Fix
* **`src/app/mod.rs`**: Altered `AppEvent::StartArchiveRun` and `AppEvent::PromptResumeResult(true)` spawned tasks to independently verify whether the `source_channel` OR the `dest_group` lacked an `InputPeer` cache entry. `get_joined_channels()` and `get_joined_groups()` are now selectively fired to ensure both ends of the pipeline exist in memory.
* **`src/archive/mod.rs`**: Moved the file creation blocks inside `if needs_download { ... }` so already-downloaded files are not completely truncated again on startup. Added an explicit `drop(file)` statement immediately after the loop to synchronously close the file handle and force OS flush before attempting the upload.

## Verification
* `cargo fmt -- --check` exited cleanly.
* `cargo clippy -- -D warnings` ran cleanly.
* `cargo build --release` succeeded.
* `cargo test` unit tests passed successfully.
