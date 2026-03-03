# Summary - Replace Magic String Auto-Topic Sentinel with Typed Boolean

Successfully refactored the auto-topic creation logic to use a typed boolean flag in the application state, removing the fragile magic-string sentinel.

## Changes

### `src/state/mod.rs`
- Added `auto_create_topic: bool` to the `State` struct with `#[serde(default)]` to ensure backward compatibility.
- Updated `test_round_trip` to include the new boolean field.
- Updated `test_migration_compatibility` to verify that old state files correctly default the new field to `false`.

### `src/app/mod.rs`
- Updated the `'s'` key handler to validate topic selection using `state.auto_create_topic` instead of magic string matching.
- Updated the topic selection logic in the TUI:
    - Choosing "Create new topic automatically" now sets `auto_create_topic = true` and clears `dest_topic_id` and `dest_topic_title`.
    - Selecting an existing topic sets `auto_create_topic = false`.
- Updated `StartArchiveRun` event handler:
    - Replaced the complex magic string extraction logic with a simple check for `state_clone.auto_create_topic`.
    - The topic title is now derived directly from `source_channel_title` (falling back to "Archive") within the async task.
    - Successfully updates `dest_topic_id`, `dest_topic_title`, and resets `auto_create_topic = false` after creation, followed by an atomic state save.

## Verification Results
- `rg "\[Auto-create topic:" repo/src` returned zero results.
- `cargo fmt -- --check`: Passed.
- `cargo clippy -- -D warnings`: Passed.
- `cargo build --release`: Passed.
- `cargo test`: Passed (2 tests passed, 1 ignored).
