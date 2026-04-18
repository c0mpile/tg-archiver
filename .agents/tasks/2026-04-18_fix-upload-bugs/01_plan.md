# Plan - Fix Upload Bugs

Two bugs in the local file upload feature need fixing in `src/app/mod.rs`.

## Bug 1: Pause and cancel are non-functional in UploadProgress view

### Problem
In `App::handle_event` for `AppEvent::StartUploadRun`, the watch channel senders for pause and cancel are created but their sender halves are dropped immediately, making `p` and `q`/`Esc` in `ActiveView::UploadProgress` inert.

### Solution
1. Add `upload_pause_tx: Option<tokio::sync::watch::Sender<bool>>` and `upload_cancel_tx: Option<tokio::sync::watch::Sender<()>>` to `App` struct.
2. Initialize them in `App::new`.
3. Store them in `StartUploadRun` handler.
4. Update `ActiveView::UploadProgress` input handler to use these senders.
5. Clear them on `UploadComplete` and `UploadError`.

## Bug 2: `AppEvent::TopicCreated` reused in upload flow causes wrong view transition

### Problem
`AppEvent::TopicCreated` handler transitions to `ActiveView::ArchiveProgress`, which is incorrect for the upload flow.

### Solution
1. Add `UploadTopicCreated(i32, String)` to `AppEvent`.
2. Update `ActiveView::UploadTopicNameEntry` handler to send `UploadTopicCreated`.
3. Add handler for `UploadTopicCreated` that transitions to `ActiveView::UploadProgress` and starts the upload run.

## Proposed Changes

### `src/app/mod.rs`

- Update `App` struct.
- Update `App::new`.
- Update `AppEvent` enum.
- Update `App::handle_event` for:
  - `AppEvent::StartUploadRun`
  - `AppEvent::UploadTopicCreated` (new)
  - `AppEvent::UploadComplete`
  - `AppEvent::UploadError`
- Update `App::handle_input` for `ActiveView::UploadTopicNameEntry` (Enter key).
- Update `App::handle_input` for `ActiveView::UploadProgress` (`p`, `q`, `Esc` keys).

## Verification Plan

1. `cargo fmt -- --check`
2. `cargo clippy -- -D warnings`
3. `cargo build --release`
4. `cargo test -- --test-threads=1`
