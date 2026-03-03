# Plan: TUI View for Archive Progress

1. Define `ArchiveProgressState` in `src/app/mod.rs` or directly add fields to `App` struct to track logs, completion status, and scroll position.
2. In `src/app/mod.rs`, add new `AppEvent` variants:
   - `ArchiveLog(String)`
   - `ArchiveStarted { start_id: i32, highest_msg_id: i32 }`
3. Modify `src/archive/mod.rs` to emit `ArchiveLog` and `ArchiveStarted` appropriately. Emit `ArchiveLog` for flood waits and chunks. Handle the empty chunk case.
4. Update `src/app/mod.rs` -> `handle_event` to match and process these new events, storing logs with timestamps (or adding timestamps when displaying). Handle the completion state staying on the screen until `q` or `r` is pressed.
5. In `src/tui/archive_progress.rs`, implement the requested layout:
   - Make the log section scrollable.
   - Bind `Up/Down` and `PageUp/PageDown` to scrolling the logs.
   - Show status bar formatted strings depending on whether the archive is active or complete.
