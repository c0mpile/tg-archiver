# Monitoring Mode Polling Loop (Phase 2) Plan

## Goal
Implement a polling loop that sequentially runs the archive process for all configured `channel_pairs` to monitor for new messages and keep destination topics synchronized.

## Module Layout and New Files
- **`src/monitor/mod.rs`**: New module housing the monitoring loop logic. It will contain a function `start_monitoring_loop(state, telegram_client, tx, cancel_token)` that spawns a `tokio` task. This task will use `tokio::time::interval` to wake up every `poll_interval_secs` (clamped to a minimum of 60 seconds).
- **`src/tui/monitoring.rs`**: New module containing the UI rendering logic for `ActiveView::Monitoring` to display the table of pairs, intervals, countdowns, and key bindings.

## Files to Modify
- **`src/app/mod.rs`**:
  - Add `ActiveView::Monitoring` to `ActiveView`.
  - Add new events: `AppEvent::MonitoringTick`, `AppEvent::PairSynced { pair_index: usize, last_forwarded_message_id: i32 }`.
  - Add `monitoring_cancel_token: Option<tokio_util::sync::CancellationToken>` (or a `watch::Sender<bool>`) to `App` for lifecycle management.
  - Add keybinding handling for the monitoring view (`a`, `d`, `s`, `i`, `q`) and `m` from the home screen.
- **`src/state/mod.rs`**:
  - Add `poll_interval_secs: u64` to `State` with `#[serde(default = "default_poll_interval")]`.
- **`src/tui/mod.rs`**:
  - Expose `pub mod monitoring;` and dispatch `ActiveView::Monitoring` in the main `render` match statement.
- **`src/archive/mod.rs`**:
  - Expose `run_archive_loop` or extract a reusable awaitable core function so that `monitor::mod.rs` can `await` it directly per pair. Currently, `start_archive_run` spawns a detached task, which makes running pairs strictly sequentially difficult. Awaiting `run_archive_loop` inside the monitoring loop solves this elegantly.

## Monitoring Loop Lifecycle
- **Start**: When the user presses `m` from the home screen, they enter `ActiveView::Monitoring`. A cancellation token is created. The monitoring loop task is spawned, passing the token, `State`, and an `mpsc::Sender` for UI updates.
- **Tick**: On each interval tick, an `AppEvent::MonitoringTick` is sent to update the UI (and potentially reset the countdown timer).
- **Execution**: The loop iterates through `state.channel_pairs`. For each pair, it `await`s the core archive logic (to enforce sequential processing). After each pair completes, it sends `AppEvent::PairSynced`, which triggers an atomic `State::save()` via the `.tmp` -> rename pattern.
- **Cancel**: When the user presses `q` to exit the monitoring view, the cancellation token is triggered. The loop checks this token before processing each pair and between intervals, allowing it to gracefully exit.

## Risks and Open Questions
1. **Reusing `start_archive_run` vs `run_archive_loop`**: 
   Since `start_archive_run` spawns a tokio task, calling it for each pair would process them concurrently, violating the rate-limit constraints. We must `await` the inner `run_archive_loop` directly in the monitoring loop. **Is it acceptable to make `run_archive_loop` public so `monitor::mod.rs` can await it sequentially?**
2. **Countdown Timer UI**: 
   A countdown requires frequent UI updates (e.g., every second). We can achieve this by having a local `tick` event in the app loop or having the monitor module send countdown ticks. Given ratatui's nature, an `AppEvent::Tick` might already exist or we can spawn a simple 1-second ticker task while in the monitoring view.
3. **AppEvent Clashes**: 
   `run_archive_loop` currently sends `AppEvent::SaveCursor` and `AppEvent::ArchiveTotalCount`. When running in monitoring mode, we must ensure these events don't unexpectedly shift the UI to `ArchiveProgress` or overwrite state incorrectly. We might need a flag to differentiate "interactive" vs "background" runs.

Please review and approve this plan.
