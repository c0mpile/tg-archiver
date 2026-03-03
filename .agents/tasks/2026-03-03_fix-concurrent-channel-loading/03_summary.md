# Task Summary: Fix Concurrent Channel Loading Race Condition

## Bug Description
The application allowed multiple concurrent state loading operations if the user pressed `Enter` more than once in the `ChannelSelect` view before `AppEvent::ChannelStateLoaded` was received. This could lead to conflicting state writes and inconsistent UI states.

## Root Cause
No guard mechanism existed in the `Enter` key handler for `ActiveView::ChannelSelect` to prevent spawning multiple Tokio tasks for state loading.

## Fix Implemented
1.  **Ephemeral Guard:** Added `channel_loading: bool` field to the `App` struct.
2.  **Guarded Input:** In the `Enter` handler for `ActiveView::ChannelSelect`, checked if `channel_loading` is true and returned early if so.
3.  **State Management:** Set `channel_loading = true` immediately before spawning the load task.
4.  **Reset Mechanism:** Set `channel_loading = false` in both `AppEvent::ChannelStateLoaded` (success) and `AppEvent::ArchiveError` (failure) handlers to ensure the app remains responsive.

## Verification
-   `cargo fmt -- --check`: Passed
-   `cargo clippy -- -D warnings`: Passed
-   `cargo build --release`: Passed
-   `cargo test -- --test-threads=1`: Passed (Note: sequential tests were used to avoid pre-existing race conditions in tests themselves).
