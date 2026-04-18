# Task: Refactor ChannelPair to use Option<i64>

In `src/state/mod.rs`, the `ChannelPair` struct uses `0` as a sentinel for unset IDs. This task refactors it to use `Option<i64>` for better type safety and consistency with the rest of the application.

## Proposed Changes

### `src/state/mod.rs`
- Update `ChannelPair` struct:
    - `source_channel_id: i64` -> `pub source_channel_id: Option<i64>` (with `#[serde(default)]`)
    - `dest_group_id: i64` -> `pub dest_group_id: Option<i64>` (with `#[serde(default)]`)
- Update `test_state_roundtrip` to assert `None` for default IDs.

### `src/app/mod.rs`
- Update validation and logic that checks for `0` sentinel.
- Use `.is_none()` for checks and `.ok_or_else(...)` for accessing IDs.

### `src/archive/mod.rs`
- Update `run_archive_loop` to handle `Option<i64>` IDs in `ChannelPair`.

## Verification Plan

### Automated Tests
- Run `cargo fmt -- --check`
- Run `cargo clippy -- -D warnings`
- Run `cargo build --release`
- Run `cargo test -- --test-threads=1`

### Manual Verification
- None required beyond automated suite.
