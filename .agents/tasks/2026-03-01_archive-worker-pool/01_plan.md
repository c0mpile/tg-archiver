# Subtask 6: Archive Worker Pool and Content Scanner

## Overview
Implement the core archive logic using a bounded parallel worker pool to download media from the source channel, apply filters, and report progress back to the TUI.

## Proposed Changes

### 1. `src/archive/mod.rs`
- **[NEW] Worker Pool & Scanner**:
  - Implement `start_archive_run` which takes `AppState`, `Arc<TelegramClient>`, and `mpsc::Sender<AppEvent>`.
  - Initialise a `tokio::sync::Semaphore` with `DEFAULT_CONCURRENCY = 3` (hard ceiling 5).
  - Use `client.iter_messages(chat)` to scan the source channel. Although `iter_messages()` fetches batches implicitly under the hood, we will maintain a local counter: after every 100 messages processed, we update and persist the `message_cursor` in `state.json` to the highest message ID processed in that batch, and then apply a 500ms `tokio::time::sleep` before requesting the next message. This ensures the 500ms delay is spacing out our consumption rate and inherently pacing the underlying batched network requests to Telegram without delaying every single message.
  - Implement filtering logic using `grammers_client::types::Media`. Check MIME types and sizes according to the user's `State::filters`.
  - Skip files below `min_size_bytes` (status: `Skipped`).
  - For matched media, spawn a worker task that acquires the semaphore, downloads the file using chunked streaming (`client.iter_download`) to `tokio::fs::File`, and sends `AppEvent::DownloadProgress` updates.

### 2. `src/app/mod.rs`
- **[MODIFY] AppEvent**:
  - Add `StartArchiveRun`, `DownloadProgress { msg_id: i32, status: DownloadStatus }`, `ArchiveComplete`, `ArchiveError(String)`.
- **[MODIFY] ActiveView & State**:
  - Add `ActiveView::ArchiveProgress`.
  - Handle the start run command (e.g., pressing `Enter` or `s` from Home view to start).
  - If `local_download_path` is `/tmp`, display a confirmation/warning prompt before starting.
- **[MODIFY] Code Cleanup**:
  - Remove all `#[allow(dead_code)]` attributes from `App` and its methods.

### 3. `src/tui/mod.rs` & `src/tui/archive_progress.rs`
- **[NEW] `ArchiveProgress` View**:
  - Show overall progress (messages scanned vs total, if available, or just a spinner/count).
  - List active downloads and their per-file status (`Pending`, `InProgress { bytes }`, `Complete`, `Failed`, `Skipped`).
  - Add logic to intercept starting a run with `/tmp` path and visually flag or prompt.

### 4. `src/telegram/mod.rs` & `src/error.rs`
- **[MODIFY] Code Cleanup**:
  - Remove `#[allow(dead_code)]` from `TelegramClient` and `AppError`.

## Verification Plan
1. `cargo fmt -- --check`
2. `cargo clippy -- -D warnings`
3. `cargo build --release`
4. `cargo test`
5. Ensure `01_plan.md`, `02_terminal.log`, and `03_summary.md` are correctly generated for this task in `.agents/tasks/2026-03-01_archive-worker-pool/`.
6. Visually test the TUI `/tmp` warning logic.
