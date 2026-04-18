# Task Summary — Fix Archive Progress Denominator

## Problem
The archive progress view in the TUI was showing a denominator of `0` for the total message count (e.g., `Forwarded ID: 2000 / 0`). This was caused by the `source_message_count` field on the `App` struct never being populated during an archive run, even though the archive worker dynamically determines the highest message ID at the start of its loop.

## Fix
1.  **Updated `AppEvent`**: Added `ArchiveTotalCount(i32)` to communicate the total message count from the background worker to the main TUI thread.
2.  **Updated Archive Worker**: Modified `run_archive_loop` in `src/archive/mod.rs` to send the `ArchiveTotalCount` event as soon as the `highest_msg_id` is determined.
3.  **Updated App State**: Added a handler in `App::handle_event` for `AppEvent::ArchiveTotalCount` that updates `self.source_message_count`.
4.  **Verified TUI**: Confirmed that `src/tui/archive_progress.rs` correctly reads `app.source_message_count` to display the progress denominator.

## Verification
- `cargo fmt -- --check`: Passed
- `cargo clippy -- -D warnings`: Passed
- `cargo build --release`: Passed
- `cargo test -- --test-threads=1`: Passed (2 tests passed, 1 ignored)

The progress view will now correctly display the total message count once an archive run starts.
