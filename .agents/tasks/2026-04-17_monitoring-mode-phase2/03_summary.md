# Phase 2 Monitoring Mode

## Implementation Overview
Successfully implemented the `Monitoring` mode polling loop as planned, enforcing strict sequential scanning to prevent rate-limiting and adding the required views and controls to the UI.

### Key Changes
- **Module `src/monitor/mod.rs`**: Created the new monitoring loop using `tokio::time::interval`, dispatching the `run_archive_loop` synchronously for each channel pair. It is fully cancellable using a `tokio::sync::watch` channel.
- **Background Archive Runs**: Updated `run_archive_loop` in `src/archive/mod.rs` to accept a `background: bool` parameter. When set to `true`, it suppresses sending `AppEvent`s that would force the TUI into the `ArchiveProgress` view, ensuring silent operation.
- **New AppEvents**: Added `MonitoringTick`, `PairSynced`, and `PairError` to handle the asynchronous communication back to the UI. The UI cleanly saves the cursor position independently on `PairSynced` without switching context.
- **Monitoring View (`src/tui/monitoring.rs`)**:
  - Shows a scrollable table displaying `Source`, `Destination`, and `Last ID` with a countdown to the next poll tick.
  - Implemented shortcut keys: `m` from Home to enter monitoring. From monitoring: `a` (add pair), `d` (delete pair prompt), `s` (force sync), `i` (interval config), and `q` (quit).
- **State Modifications (`src/state/mod.rs`)**:
  - Added `poll_interval_secs: u64` with a `serde(default)` fallback to 300 seconds (5 minutes). 

### Verification Completed
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo build --release`
- `cargo test -- --test-threads=1`

All changes successfully adhere to the project's strict architecture constraints and XDG pathing standards.
