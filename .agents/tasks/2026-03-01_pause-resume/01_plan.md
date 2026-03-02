# Goal Description
Implement Pause/Resume Architecture and State Persistence (Subtask 9). The app will allow toggling pause from the TUI which gracefully stops new worker spawns without cancelling active ones. The state is atomically flushed on pause. On startup, incomplete archives (Pending/InProgress) will trigger a TUI prompt to resume or start fresh.

## Cursor Direction and Resume Mechanics (Clarification)
- **Iteration Direction:** `grammers_client::Client::iter_messages()` retrieves messages **newest-first** (in descending ID order).
- **Cursor Value:** During a run, `message_cursor` is continuously updated to the **lowest (oldest) message ID** yielded by the iterator in the current chunk. After a partial run, `message_cursor` securely holds the ID of the oldest message fetched so far.
- **Resume Offset:** On Resume, `telegram_client.client.iter_messages(peer).offset_id(state.message_cursor.unwrap())` is used. Since `offset_id(X)` tells Telegram to fetch messages *older* than `X`, passing the lowest processed ID successfully skips all already-processed ascending messages. (Note: A consequence of a strict resume is that any *new* messages arriving during the pause are ignored in this run to prevent re-scanning; they are captured on the next "Fresh Start").

## Fresh Start File Handling (Clarification)
- **State Reset:** When the user chooses **Start Fresh** at the resume prompt, the application clears the `download_status` map and resets the `message_cursor` to `None`.
- **File System Handling:** Existing partial or completed files in the local download directory are **left in place** (they are not proactively deleted).
- **Overwrite Behavior:** As the "Fresh Start" archiver scans messages, if it encounters a file it needs to download, `tokio::fs::File::create(&file_path)` will implicitly overwrite any existing file at that path (whether partial or complete). The duplicate is ignored, and the new contents are cleanly downloaded from scratch over the old file.

## Proposed Changes

### src/app/mod.rs
- **Add states and events:** `AppEvent::TogglePause`, `AppEvent::PromptResumeResult(bool)`
- **Add view:** `ActiveView::ResumePrompt`
- **TUI initialization:** In `App::new`, if `state.message_cursor.is_some()` or there are any `Pending` or `InProgress` downloads, default `active_view` to `ResumePrompt` instead of `Home`.
- **Handle events:** 
  - In `ArchiveProgress` view, pressing `p` or space triggers `AppEvent::TogglePause`.
  - When paused, toggle `app.is_paused` and trigger an atomic flush `state.save().await`.
  - In `ResumePrompt` view, 'y' transitions to `ArchiveProgress` and dispatches `StartArchiveRun`; 'n' clears `download_status` and `message_cursor` and transitions to `Home`.
- **App struct fields:** Add `pub is_paused: bool` to the `App` state.
- **State passing:** Create `Arc<AtomicBool>` for the pause state and pass a clone into `start_archive_run`.

### src/archive/mod.rs
- **Worker pool modifications:** `start_archive_run` and `run_archive_loop` will accept an `Arc<AtomicBool>` for `pause_flag`.
- **Pause logic:** Inside `run_archive_loop`, before retrieving the next message or spawning a new download task, loop while the pause flag is true:
    ```rust
    while pause_flag.load(std::sync::atomic::Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    ```
- **Resume offset:** If `state.message_cursor.is_some()`, add `.offset_id(state.message_cursor.unwrap())` to `telegram_client.client.iter_messages(...)` to resume from the oldest message ID processed in the previous session.
- **Cursor tracking:** Change the `highest_msg_id` tracker to a `lowest_msg_id` tracker. Initialize it as `state.message_cursor.unwrap_or(i32::MAX)`. For each message, if `msg_id < lowest_msg_id`, update `lowest_msg_id = msg_id`. Save this as the `message_cursor` to accurately reflect the oldest point to resume from.

### src/state/mod.rs
- **State Load:** Modify `State::load()` to iterate through all values in `download_status` and convert any `InProgress` entries to `Pending`, enforcing that partial downloads are restarted from scratch.

### src/tui/mod.rs & src/tui/archive_progress.rs
- **TUI updates:** `archive_progress::draw` will check `app.is_paused` and explicitly display `[PAUSED]` if true.
- **Resume Prompt Renderer:** Add `render_resume_prompt` to `src/tui/mod.rs` to visually prompt the user for Resume (y/n) at startup.

### src/telegram/mod.rs
- **Cleanup:** We verified `#[allow(dead_code)]` on `_pool_task` has already been removed, so no action needed.

## Verification Plan

### Automated Tests
- Run `cargo fmt -- --check`
- Run `cargo clippy -- -D warnings`
- Run `cargo build --release`
- Run `cargo test`

### Manual Verification
- Start an archive via TUI, observe it fetching messages.
- Press `p` to pause the archive. Wait for active downloads to gracefully finish.
- Verify `~/.local/state/tg-archiver/state.json` updates and reflects exact state immediately upon pause.
- Shutdown/force quit the app.
- Restart the app. Expect the `ResumePrompt` TUI.
- Press 'y' to resume. Observe it continuing from the previously stopped point, skipping processed messages without re-downloading them.
