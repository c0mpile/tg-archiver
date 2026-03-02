# Upload Bug Fix Plan

## Root Cause Analysis
Following the investigation order requested:

1. **`needs_upload` check triggering:** The `needs_upload` check *does* correctly trigger if the dest group is set and the status is not already `Uploaded`. However, there is a major logical flaw: the `tokio::fs::File::create` call and the actual chunk download loops for images/documents execute unconditionally in the worker block, *even if `needs_download` is false*. That means files that were already completed and resumed are re-truncated and re-downloaded before the upload runs.

2. **`upload_media` calling:** `upload_media` is actually called if `needs_upload` is true, *provided* the destination peer is found in the cache. 

3. **`peer_cache` presence (The Main Silencer):** If a user selects a source channel in the TUI during the session (which populates the cache with channels), but doesn't select the destination group (relying on previously saved state), `StartArchiveRun` sees that the source is already in the cache (`tg_clone.get_input_peer(source_id).await.is_none()` evaluates to `false`). It then **completely skips** warming up the `get_joined_groups()` cache. Thus, during upload, the cache lookup for the destination group ID silently returns `None` and aborts the upload, throwing a "Destination peer not found in cache" error.

4. **Upload error swallowing:** Upload errors are actually correctly bubbled up from `retry_flood_wait!` and sent as `DownloadStatus::Failed` to the TUI. However, another race condition limits upload success: the `tokio::fs::File` is kept open until the end of the block. Thus, `upload_file()` attempts to read a file that the OS hasn't necessarily flushed the file size metadata for.

5. **`topic_id` format:** `grammers` handles `reply_to(Some(tid))` perfectly fine for posting into forum topics, as MTProto implements topics simply as message threads where the topic ID is just the action message ID. This is structurally correct.

6. **State Machine consistency:** After `UploadMedia` completes successfully, `DownloadStatus::Uploaded` must be dispatched to persist the state using the atomic write pattern. `src/app/mod.rs` handles `AppEvent::DownloadProgress` correctly, which invokes `state.save()` correctly.

## Proposed Changes

### `src/app/mod.rs`
#### [MODIFY] src/app/mod.rs
* Update the `AppEvent::StartArchiveRun` and `PromptResumeResult(true)` logic to independently check if the source peer AND destination peer are missing from the cache, warming the respective missing cache instead of skipping everything if only the source is present.

### `src/archive/mod.rs`
#### [MODIFY] src/archive/mod.rs
* Place the `tokio::fs::File::create` and the associated download blocks (`if is_photo { ... } else if let Some(...)`) *inside* the `if needs_download { ... }` guard.
* Add an explicit `drop(file)` statement at the end of the download logic inside the `if needs_download` guard, forcing an OS-level file flush so that `upload_media` can properly stat and transmit the completed file.
* Confirm that `AppEvent::DownloadProgress { status: DownloadStatus::Uploaded }` successfully updates the state reliably using the central event loop in `src/app/mod.rs`.

## Verification Plan

### Automated Tests
Run `cargo check` and `cargo test` to ensure standard build checks pass.

### Manual Verification
Reviewing the structure ensures logical flows no longer conflict. The combination of explicitly dropping the file descriptors and intelligently warming the peer caches will unblock the upload functionality while respecting skip boundaries.
