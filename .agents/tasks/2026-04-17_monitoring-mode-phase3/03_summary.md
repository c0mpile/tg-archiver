# Phase 3 Monitoring Mode: Add Status Column

## Summary
Completed Phase 3 of monitoring mode.

Added a `PairStatus` enum to `App` to track `Idle`, `Syncing`, and `Error` states for each configured channel pair. The monitoring loop now dispatches a `PairSyncStarted` event before running the `run_archive_loop` for a pair, moving it into `Syncing` state. After success or error, it falls back to `Idle` or `Error(msg)`. The monitoring UI layout was updated to include the status as a fourth column and rendering full error strings dynamically inside a one-line layout footer when the user highlights the failed pair.

## Tasks Completed
- Updated `src/app/mod.rs` to track and transition `pair_statuses`.
- Dispatched `PairSyncStarted` from the background task in `src/monitor/mod.rs`.
- Rendered status strings with distinct coloring (Idle=DarkGray, Syncing=Yellow, Error=Red) in `src/tui/monitoring.rs`.
- Passed format, clippy, and unit tests locally.
