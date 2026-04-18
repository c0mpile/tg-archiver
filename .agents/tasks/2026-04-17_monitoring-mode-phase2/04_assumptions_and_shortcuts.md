# Monitoring Mode Phase 2: Assumptions and Shortcuts

## Assumptions

1. **`AppEvent::PairError` handling**: I added an explicit `AppEvent::PairError` to be sent when a background chunk fails. However, I deliberately left the handling of this event empty in the main event loop (`src/app/mod.rs`). I assumed that failing silently in the background and retrying automatically on the next interval is acceptable for now. The error isn't surfaced to the UI to avoid distracting popup modals during unattended monitoring.

2. **Dummy Pause Flag**: I passed a dummy atomic boolean (`false`) to `run_archive_loop` when it's running in background mode. I assumed that the global pause functionality (`TogglePause`) is intended only for foreground, interactive archive runs, and that the background monitor should not pause but should either run or be fully exited via `q`.

3. **Approximating `next_tick_at`**: Instead of adding a dedicated background countdown timer task, I approximate the next tick by calculating `Instant::now() + poll_interval_secs` upon entering the view and on each `MonitoringTick`. This simplifies the architecture by utilizing ratatui's natural render loop to calculate the remaining seconds dynamically. 

4. **Pair Deletion Behavior**: In `ActiveView::DeletePairPrompt`, I save the modified state immediately to disk without further confirmation upon pressing `y` and automatically adjust the `active_pair_index` bounds. I assumed no undo logic is necessary.

5. **`last_forwarded_message_id` Syncing**: To avoid emitting generic `SaveCursor` events that would confuse the TUI state, I changed `run_archive_loop` to return its final `last_forwarded_message_id`. The monitor loop receives this value upon completion and explicitly sends `PairSynced` so the UI can update accurately and atomically. 

## Shortcuts

1. **Cancellation implementation**: Rather than adding the external `tokio-util` crate to use a `CancellationToken`, I used the standard `tokio::sync::watch::channel(false)`. This natively serves as an effective, dependency-free cancellation mechanism to break out of the interval loop or between pair iterations without external library overhead.

2. **Reusing interactive workflows**: For the "add pair" shortcut (`a`) in the Monitoring view, I simply appended an empty pair to the state and pushed the user into the existing `ChannelSelect` view. I chose to kill the monitoring loop during this flow. The user will be deposited on the `Home` view once the new pair setup completes, rather than returning to `Monitoring`. This avoids having to create a separate "background setup wizard" state machine.

3. **Force Sync shortcut**: Pressing `s` from the Monitoring view forcefully exits the monitor loop and pushes the UI into the foreground `ArchiveProgress` view using the existing `StartArchiveRun` flow. This avoids complex concurrency locks that would be required if we allowed a foreground forced run and a background interval run to operate on the same pair simultaneously.
