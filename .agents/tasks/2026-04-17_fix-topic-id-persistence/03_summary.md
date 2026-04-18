# Summary — Topic ID Persistence Bug Fix

## Root Cause
The `App` struct's `self.state` was not being updated when a topic was auto-created in a background task. Subsequent state saves (from cursor updates or pauses) used the stale `self.state`, overwriting the disk with `dest_topic_id: None`.

## Fix
Implemented a message-passing fix using a new `AppEvent::TopicCreated` variant. This ensures the main thread updates its own state when a topic is created, maintaining synchronization. Also improved state robustness by adding `#[serde(default)]` to optional fields in `ChannelPair`.

## Verification
- Full test suite passed.
- Clippy and formatting checks passed.
- Verified `serde` behavior for missing fields via unit test.
