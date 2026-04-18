# Summary - Fix Upload Bugs

Fixed two bugs in the local file upload feature in `src/app/mod.rs`.

## Root Cause & Fix

### Bug 1: Pause and cancel are non-functional in UploadProgress view
- **Root Cause:** The pause and cancel watch channel senders were created but not persisted, causing them to be dropped and the receivers to become inactive.
- **Fix:** Added `upload_pause_tx` and `upload_cancel_tx` fields to the `App` struct. These are now populated during `AppEvent::StartUploadRun` and utilized in the `ActiveView::UploadProgress` input handler to signal pause/resume and cancellation. They are cleared on upload completion or error.

### Bug 2: `AppEvent::TopicCreated` reused in upload flow causes wrong view transition
- **Root Cause:** The `AppEvent::TopicCreated` handler was hardcoded to transition to `ActiveView::ArchiveProgress`, which broke the UI flow when creating a topic for local file uploads.
- **Fix:** Introduced a new `AppEvent::UploadTopicCreated(i32, String)` variant specifically for the upload flow. The `ActiveView::UploadTopicNameEntry` handler now sends this new event, and its handler transitions correctly to `ActiveView::UploadProgress` before triggering the upload run.

## Verification Results

- `cargo fmt -- src/app/mod.rs` (passed)
- `cargo clippy -- -D warnings` (passed)
- `cargo build --release` (passed)
- `cargo test -- --test-threads=1` (passed: 2 passed, 1 ignored)

All changes are restricted to `src/app/mod.rs` as per the defined scope.
