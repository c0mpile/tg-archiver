# Plan — Fix Topic ID Persistence Bug

## 1. Characterize the Bug
- Resume from pause or app restart causes messages to go to `#General`.
- This implies `dest_topic_id` is `None` in the loaded state.
- Root cause suspected: `App` state is stale after auto-topic creation, and subsequent saves overwrite the disk with `None`.

## 2. Investigation
- [x] Trace `dest_topic_id` updates in `src/app/mod.rs`.
- [x] Check `SaveCursor` handler for state synchronization.
- [x] Verify `serde` deserialization behavior for missing fields.

## 3. Implementation
- [x] Add `AppEvent::TopicCreated` to propagate auto-created IDs to the main thread.
- [x] Update `self.state` in `App` when the event is received.
- [x] Ensure `auto_create_topic` is set to `false` in the main state.
- [x] Add `#[serde(default)]` to all optional fields in `ChannelPair` for robustness.

## 4. Verification
- [x] Run `cargo fmt -- --check`.
- [x] Run `cargo clippy -- -D warnings`.
- [x] Run `cargo build --release`.
- [x] Run `cargo test -- --test-threads=1`.
