# Task Summary — Fix Auto-create Topic Validation

The issue where "Create Automatically" was incorrectly rejected by pre-flight validation has been fixed.

## Root Cause
The `s` key handler in `ActiveView::Home` (pre-flight validation) checked for `self.state.dest_topic_id.is_none()` but didn't allow for the auto-create sentinel string. This caused the app to report "Missing configuration: Destination Topic" despite the "Create Automatically" option being selected.

## Fix
- **Updated Sentinel String:** Changed the sentinel marker from `"<Create Automatically>"` to a more descriptive `"[Auto-create topic: CHANNEL_TITLE]"`. This avoids potential collisions and allows for easier title extraction.
- **Improved Validation:** Updated the `s` key pre-flight check to bypass the "Missing Topic" error when `dest_topic_title` matches the auto-generate sentinel.
- **Dynamic Topic Generation:** Enhanced the `AppEvent::StartArchiveRun` handler to extract the topic title directly from the sentinel string and use it when calling the Telegram API's `create_topic`.
- **Atomic State Update:** Ensured that once the topic is created, the `dest_topic_id` is persisted immediately before starting the archive run.

## Verification
- Verified code structure in `src/app/mod.rs`.
- Ran `cargo check` and `cargo fmt` to confirm stylistic and syntactic correctness.
- Ran `cargo test` to ensure existing tests still pass.
- Successfully built the release binary.
