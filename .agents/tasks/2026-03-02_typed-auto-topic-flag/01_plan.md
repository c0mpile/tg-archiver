# Plan - Replace Magic String Auto-Topic Sentinel with Typed Boolean

Currently, the application uses a magic string `"[Auto-create topic: ..."` in `dest_topic_title` to signal that a new topic should be created automatically. This is fragile and should be replaced with a proper boolean flag in the `State`.

## User Requirements
1. Add `auto_create_topic: bool` to the `State` struct in `src/state/` with `#[serde(default)]`.
2. Update TUI to set `auto_create_topic = true` and clear topic fields when "Create Automatically" is chosen.
3. Update `'s'` key handler and `StartArchiveRun` event to use this flag.
4. Remove all magic string occurrences.
5. Update tests.

## Proposed Changes

### 1. `src/state/mod.rs`
- Add `pub auto_create_topic: bool` to `State` struct.
- Update `test_round_trip` to include the new field.
- Update `test_migration_compatibility` to verify it defaults to `false`.

### 2. `src/app/mod.rs`
- **`handle_event` (KeyCode::Char('s'))**:
    - Replace the `map` on `dest_topic_title` that checks for the magic string with a direct check of `self.state.auto_create_topic`.
- **`handle_event` (ActiveView::TopicSelect -> KeyCode::Enter)**:
    - When `i == 0`, set `self.state.auto_create_topic = true`.
    - Set `self.state.dest_topic_id = None`.
    - Set `self.state.dest_topic_title = None`.
- **`handle_event` (AppEvent::StartArchiveRun)**:
    - Replace the multi-line `if` condition that checks `dest_topic_title` magic string with a check for `state_clone.auto_create_topic`.
    - Derive `topic_title` from `state_clone.source_channel_title` (defaulting to "Archive").
    - After `tg_clone.create_topic` succeeds:
        - Set `state_clone.dest_topic_id = Some(new_topic_id)`.
        - Set `state_clone.dest_topic_title = Some(topic_title.to_string())`.
        - Set `state_clone.auto_create_topic = false`.
        - Save the updated `state_clone`.

## Verification Plan

### Automated Tests
- Run `cargo test` to ensure state serialization and logic are correct.

### Linter and Formatter
- Run `cargo fmt -- --check`.
- Run `cargo clippy -- -D warnings`.

### Manual Build
- Run `cargo build --release`.

### Final Check
- Run `rg "\[Auto-create topic:" repo/src` to ensure zero occurrences.
