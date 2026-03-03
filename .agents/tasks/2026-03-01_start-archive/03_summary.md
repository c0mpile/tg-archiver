# Start Archive Run Validation Summary

Added validation checks when starting an archive run from the Home screen via the 's' key. 

## Changes Made
- **state/App**: Unlocked `home_error: Option<String>` on the `App` struct.
- **Home View Event Handler**: Pressing 's' on the home screen now explicitly validates that `source_channel_id`, `dest_group_id`, and `dest_topic_id` are configured.
    - If any of these fields are missing, `home_error` is populated with a comma-separated list of the missing settings and the user is held on the Home view.
    - If all fields exist, it clears `home_error` and correctly checks for `/tmp` confirmation, or starts the download immediately by dispatching `AppEvent::StartArchiveRun`.
- **TUI Updates**: Modified `src/tui/mod.rs` to clearly show "Press 's' to start archive." on the home screen, and rendered inline error messages in red bolded text whenever `app.home_error` is populated.

## Quality Checks
- Rust compilation (release build) succeeds inline without warnings.
- `cargo fmt` and `cargo clippy` passed cleanly.
- Tests passing.
- Followed "Surgical Modification" requirements without excessive refactors.
