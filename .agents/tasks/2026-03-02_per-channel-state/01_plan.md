# Per-Channel State Implementation Plan

## Approach

The goal is to migrate from a single monolithic `state.json` file to per-channel state files (`state-{source_channel_id}.json`).

1. **State Loading & Saving (`src/state/mod.rs`)**:
   - Update `State::load()` to take an optional `source_channel_id`. Or introduce `State::load_for_channel(id: i64)`.
   - Update `get_state_file_path()` to dynamically embed the `source_channel_id` (e.g. `state-{id}.json`). The legacy `state.json` file will simply be ignored.
   - Update `State::save()` to only save if the struct's `source_channel_id` is `Some(id)`. If `None`, do nothing since there's no valid filename.
   - Update tests to explicitly test `load` and `save` behaviour using a mocked XDG state directory pointing to `std::env::temp_dir()`.

2. **Initialization (`src/main.rs`)**:
   - Remove `State::load().await` on startup, which currently assumes a single `state.json`. Instead, initialize `App` with `State::default()`.

3. **TUI State Transition (`src/app/mod.rs`)**:
   - The TUI runs a synchronous event loop. When a user selects a source channel (hitting `Enter` on `ActiveView::ChannelSelect`), we must seamlessly load the state for that channel.
   - We will spawn a Tokio task that:
     1. Atomically saves the *current* channel's state if `state.source_channel_id.is_some()`.
     2. Calls `State::load_for_channel(new_id).await`. If the file does not exist, it naturally returns `State::default()`.
     3. Sets the new `source_channel_id` / title on the newly loaded state.
     4. Dispatches a new `AppEvent::ChannelStateLoaded(State)` to the main loop.
   - Upon receiving `AppEvent::ChannelStateLoaded`, `App::handle_event` updates `self.state`, navigates to `ActiveView::GroupSelect`, and dispatches the task to fetch groups, matching existing behaviour but with the correct per-channel state attached.

## Files Modified

### [MODIFY] src/state/mod.rs
- Add `get_state_dir()` and `get_state_file_path(id: i64)`.
- Change `State::load()` signature or add `State::load_for_channel(id: i64)`.
- Add safety checks to `State::save()` to return `Ok(())` if `source_channel_id` is missing.
- Refactor the serialization tests to write actual files in `std::env::temp_dir()` and read them back to ensure the disk paths and `.tmp` rename mechanism behave correctly.

### [MODIFY] src/main.rs
- Change `let state = state::State::load().await.unwrap();` to `let state = state::State::default();`.

### [MODIFY] src/app/mod.rs
- Add `AppEvent::ChannelStateLoaded(State)` to `AppEvent` enum.
- In `crossterm::event::KeyCode::Enter` handler for `ActiveView::ChannelSelect`:
  - Dispatch a task that saves `self.state`, loads the new channel's state from disk, and sends `AppEvent::ChannelStateLoaded(state)`.
- Implement `AppEvent::ChannelStateLoaded(new_state)` handler:
  - Assign `self.state = new_state`.
  - Transition view to `ActiveView::GroupSelect`.
  - Fetch user groups concurrently via `TelegramClient::get_joined_groups()`.

## Risks & Open Questions
- None. This approach adheres cleanly to the synchronous TUI constraints, avoids `std::fs` blocking in async functions, and preserves existing Serde guarantees and `.tmp` save patterns.

## Verification Plan

### Automated Tests
- Run `cargo test` explicitly verifying the updated `test_round_trip` unit tests covering per-channel load/save in temp directories.
- Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo build --release`.

### Manual Testing Requirements
Since this touches core initialization logic, the final behavior can optionally run in the terminal or be verified mathematically via existing tests mapping to `AppEvent`. 
