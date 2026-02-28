---
description: Implement pause and resume
---

Using the tg-archiver project rules, implement or repair the pause/resume
behaviour for an active archive session.

Expected behaviour:
- The user can pause a running archive from the progress view in the TUI.
  In-flight downloads (already started) are allowed to complete. No new
  downloads are started after the pause signal is received.
- Pausing triggers an immediate atomic state flush to
  `~/.local/state/tg-archiver/state.json` (write to `.tmp`, rename).
- On resume (same session or new session), the app reads `state.json`, skips
  any files with status `Complete` or `Skipped`, and resumes downloading from
  the first `Pending` or `InProgress` file. Files with status `InProgress`
  from a previous session are reset to `Pending` on load (partial downloads
  are not resumed at the byte level — the file is re-downloaded from the start).
- The message ID cursor in state must reflect the last fully processed message
  so that a resume does not re-scan already-processed messages from the
  beginning.

Implementation rules:
1. The pause signal is an `AppEvent` variant sent from the TUI input handler
   to `App::handle_event()`. The worker pool reads a shared
   `tokio::sync::watch` or `AtomicBool` pause flag — do not cancel tasks,
   use the flag to gate new task spawning.
2. State flush on pause must use the atomic write pattern:
   `~/.local/state/tg-archiver/state.json.tmp` → rename.
3. On startup, if `state.json` exists and contains work in a non-terminal
   state, present the user with a TUI prompt offering to resume or start fresh.
   Starting fresh must not delete `state.json` until the user confirms.
4. Do not use `unwrap()` or `expect()` anywhere in the pause/resume path.
5. Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build
   --release`, and `cargo test` — all must exit 0.