# Task: Fix Concurrent Channel Loading Race Condition

## Bug Description
In `src/app/mod.rs`, the `Enter` key handler for `ActiveView::ChannelSelect` spawns a Tokio task to save the current state and load the new channel's state. Pressing `Enter` multiple times before `AppEvent::ChannelStateLoaded` is received results in multiple concurrent tasks, causing conflicting state writes.

## Proposed Fix
1.  Add `channel_loading: bool` to `App` struct (ephemeral).
2.  Guard the `Enter` handler with `channel_loading` check.
3.  Set `channel_loading = true` when starting the load.
4.  Set `channel_loading = false` when `AppEvent::ChannelStateLoaded` is handled.

## Plan
1.  Explore `src/app/mod.rs` to identify `App` struct and relevant event handlers.
2.  Modify `App` struct definition.
3.  Update `App::new()` or equivalent initialization.
4.  Update the `ChannelSelect` `Enter` key handler.
5.  Update the `AppEvent::ChannelStateLoaded` handler.
6.  Verify the changes.
