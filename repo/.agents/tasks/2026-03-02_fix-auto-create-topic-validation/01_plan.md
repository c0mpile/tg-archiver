# Task Plan — Fix Auto-create Topic Validation

## Bug Description
When the user selects "Create Automatically" for the destination topic and presses `s` to start the archive, the application errors with "Missing configuration: Destination Topic".

## Root Cause
The pre-flight validation in the `s` key handler (`ActiveView::Home`) rejects `dest_topic_id` being `None` without checking for the auto-create sentinel in `dest_topic_title`.

## Proposed Fix
1.  **Update Sentinel Type:** Change the sentinel from `"<Create Automatically>"` to `"[Auto-create topic: CHANNEL_TITLE]"` for better identification and automatic naming.
2.  **Update Topic Select Handler:** Update the `TopicSelect` key handler to set the new sentinel and store the source channel title within it.
3.  **Update Pre-flight Validation:** Modify the `s` key handler to allow `dest_topic_id.is_none()` if the `dest_topic_title` matches the sentinel.
4.  **Update Run Loop Logic:** Update `AppEvent::StartArchiveRun` to extract the title from the sentinel and use it when calling `create_topic`.

## Verification Plan
1.  Run `cargo clippy` to ensure no warnings or errors.
2.  Run `cargo fmt` to match project style.
3.  Run `cargo build --release` to ensure compilation.
4.  Run `cargo test` to execute available tests.
5.  Manual verification of logic in `src/app/mod.rs`.
