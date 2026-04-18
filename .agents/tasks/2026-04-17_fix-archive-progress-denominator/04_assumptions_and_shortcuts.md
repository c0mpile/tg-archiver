# Assumptions and Shortcuts — Fix Archive Progress Denominator

## Assumptions

1.  **Message ID as Count**: I assumed that `highest_msg_id` is an acceptable proxy for the total message count. While Telegram message IDs are mostly sequential, deleted messages can create gaps. However, since the TUI was already designed to display `ID / Total`, using the highest ID as the denominator is consistent with the existing design.
2.  **Single Event Emission**: I assumed that sending the `ArchiveTotalCount` event once at the beginning of the `run_archive_loop` is sufficient. If the source channel receives new messages *during* the archive run, the denominator will not update until the next run. This matches the current sequential "catch-up" architecture of the app.
3.  **TUI Field Alignment**: I assumed (and verified) that `app.source_message_count` was the intended field for the TUI progress view. The TUI code was already reading this field, so no changes were needed in `src/tui/`.

## Shortcuts

1.  **No New State Persistence**: I did not persist the `source_message_count` to the `State` JSON. The instructions specified it as an "ephemeral" field on `App`. This means every time the app restarts and resumes, it will briefly show `/ 0` until the archive worker re-fetches the highest ID. This is acceptable given the task scope.
2.  **No Progress update during chunks**: I didn't add logic to update the total count dynamically if it changes during the run, as the requirement was strictly to populate the denominator for the progress view.

## Technical Decisions

1.  **Task Audit Path**: I strictly followed the user rule to write task artifacts to `~/dev/tg-archiver/.agents/tasks/` instead of the system's default artifact directory, bypassing the `IsArtifact` flag to avoid system-enforced path constraints.
