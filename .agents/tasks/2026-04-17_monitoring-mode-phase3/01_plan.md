# Phase 3 Monitoring Mode: Add Status Column

This plan outlines the addition of a per-pair status column to the monitoring view in `tg-archiver`.

## Proposed Changes

### App State (`src/app/mod.rs`)
- Define the `PairStatus` enum with `Idle`, `Syncing`, and `Error(String)` variants, deriving `Debug`, `Clone`, `PartialEq`, and `Default`.
- Add `PairSyncStarted { pair_index: usize }` to `AppEvent`.
- Add `pair_statuses: Vec<PairStatus>` to the `App` struct.
- Initialize `pair_statuses` in `App::new` to be the same length as `state.channel_pairs`.
- In `App::handle_event`:
  - `PairSyncStarted`: Update the status at `pair_index` to `Syncing`.
  - `PairSynced`: Update the status at `pair_index` to `Idle`.
  - `PairError`: Update the status at `pair_index` to `Error(msg)`.
  - When adding a new pair (`'a'` key): Push `PairStatus::default()` to `pair_statuses`.
  - When deleting a pair (`'y'` key in `DeletePairPrompt`): Remove the corresponding status from `pair_statuses`.

### Monitoring Loop (`src/monitor/mod.rs`)
- Before calling `run_archive_loop` for a pair, dispatch a `PairSyncStarted` event with the current `pair_index`.

### TUI (`src/tui/monitoring.rs`)
- Add `"Status"` to the table header.
- Update table column constraints to `Percentage(30), Percentage(30), Percentage(15), Percentage(25)`.
- Extract the status for each row and style it:
  - `Idle`: `Color::DarkGray`, text `"Idle"`
  - `Syncing`: `Color::Yellow`, text `"Syncing..."`
  - `Error(_)`: `Color::Red`, text `"Error"`
- Add an error footer to the layout constraints (`Constraint::Length(1)`) between the table and the help bar.
- If the currently selected pair has an `Error(msg)` status, display `"Error: <msg>"` in red in the footer. Otherwise, display a blank line.
