# Summary of State Schema Change

## Modifications
- Replaced flat `source_channel_id` / `dest_group_id` / `dest_topic_id` / `last_forwarded_message_id` fields in `State` with `channel_pairs: Vec<ChannelPair>`.
- Preserved JSON deserialization compatibility via `#[serde(default)]`.
- Added `active_pair_index: usize` and `source_message_count` to `App` struct.
- Refactored `App::handle_event` and `run_archive_loop` to access channel IDs via `channel_pairs[active_pair_index]`.

## Verification
- Applied `cargo fmt`.
- Migration test implemented in `src/state/mod.rs`.
- Tests and linters passed successfully.
