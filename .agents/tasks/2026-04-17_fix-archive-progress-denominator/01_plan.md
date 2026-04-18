# Task Plan — Fix Archive Progress Denominator

The archive progress view currently shows `0` as the total message count (e.g., `Forwarded up to ID: 2000 / 0`) because `source_message_count` is not being populated on the `App` struct. The archive worker knows the highest message ID but does not communicate it back to the UI.

## Proposed Changes

### 1. Update `AppEvent` Enum
- File: `src/app/mod.rs`
- Add `ArchiveTotalCount(i32)` variant to the `AppEvent` enum.

### 2. Update Archive Worker
- File: `src/archive/mod.rs`
- In `run_archive_loop`, after fetching the `highest_msg_id`, send `AppEvent::ArchiveTotalCount(highest_msg_id)` via the transmitter.

### 3. Update `App::handle_event`
- File: `src/app/mod.rs`
- Add a match arm for `AppEvent::ArchiveTotalCount(n)` that sets `self.source_message_count = Some(n)`.

### 4. Verify TUI Progress View
- File: `src/tui/` (search for progress rendering)
- Ensure it uses `app.source_message_count`.

## Verification Plan

### Automated Tests
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo build --release`
- `cargo test -- --test-threads=1`

### Manual Verification
- (If possible, though primarily relying on automated checks and surgical correctness as per instructions).
