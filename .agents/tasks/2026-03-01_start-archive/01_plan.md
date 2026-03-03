# Implementation Plan: Start Archive Validation

## Approach

The home screen already has partial support for starting an archive via the 's' key, but it lacks validation for required configuration fields (source channel, destination group, and topic) and UI guidance. This feature will add the missing validation and update the home screen UI to explicitly show the 's' key option and any resulting error messages.

## Files to be Modified

1. `src/app/mod.rs`:
   - Add `home_error: Option<String>` to the `App` struct to track start validation errors.
   - Update `App::new` to initialize `home_error` to `None`.
   - Update `App::handle_event` in the `ActiveView::Home` match arm for `KeyCode::Char('s')`:
     - Check if `self.state.source_channel_id`, `self.state.dest_group_id`, and `self.state.dest_topic_id` are `Some`.
     - If any are missing, build a comma-separated list of what is missing, set `home_error` to something like "Missing configuration: Source Channel", and do not start the archive.
     - If all are present, clear `home_error` and proceed with the existing `/tmp` check and dispatch `AppEvent::StartArchiveRun`.

2. `src/tui/mod.rs`:
   - Update `render_home` to include "Press 's' to start archive." in the menu text.
   - Update `render_home` to check if `app.home_error` is `Some`. If it is, render it as inline red text at the bottom of the home screen view.

## Risks and Open Questions

- We're adding a new error field specifically for the home view rather than repurposing `resolution_error`, which ensures state remains cleanly separated by view type.
- The `ConfirmDownloadPath` view already correctly dispatches `AppEvent::StartArchiveRun`. Wait, does it bypass validation? If the user presses 's' and the path is `/tmp`, it transitions to `ConfirmDownloadPath`. The validation in `ActiveView::Home` for 's' will run *before* making the transition to `ConfirmDownloadPath`, so they won't be able to reach the confirmation unless all required fields are set. This is correct.

## Checklist
- [ ] Add `home_error` to `App`.
- [ ] Implement validation logic on 's' press.
- [ ] Update TUI text to show 's' hotkey and error messages via ratatui spans.
- [ ] Run `cargo clippy`, `cargo fmt`, and `cargo test`.
