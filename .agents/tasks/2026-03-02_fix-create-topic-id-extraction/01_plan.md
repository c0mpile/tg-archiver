# Plan: Fix Topic ID Extraction in `create_topic()`

## Problem

In `repo/src/telegram/mod.rs`, the `create_topic()` function attempts to extract the newly created topic ID from the `CreateForumTopic` response updates by looking for the `Update::MessageId` variant. This variant is incorrect for topic creation; it's intended for outgoing message edits.

## Failure Mode

- `create_topic()` will likely fail to extract the ID from the updates list.
- It will then trigger the fallback logic (listing all topics and matching by title).
- While the fallback might work, if the fallback fails or if a race condition occurs, the function could return an error even if the topic was created.
- The extraction from updates is more efficient and reliable if done correctly.

## Root Cause

Incorrect `Update` enum variant used when parsing the response from `messages.createForumTopic`.

## Proposed Solution

Modify the update parsing logic in `create_topic()` to:

1. Iterate through `u.updates`.
2. Look for `Update::NewChannelMessage(m)` or `Update::NewMessage(m)`.
3. Verify if `m.message` is a `Message::Service`.
4. If so, use `m.message.id` as the topic ID.
5. Retain the existing fallback to `list_topics()` if the above yields nothing.

## Steps

1. Document failure mode and root cause (this plan).
2. Register the task and log the terminal output.
3. Apply surgical fix to `repo/src/telegram/mod.rs`.
4. Run verification:
   - `cargo fmt -- --check`
   - `cargo clippy -- -D warnings`
   - `cargo build --release`
   - `cargo test`
5. Document results in `03_summary.md`.
