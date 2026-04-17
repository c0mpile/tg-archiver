---
description: Implement or debug pause and resume behaviour
---

Using the tg-archiver project rules, implement or repair pause/resume
behaviour for an active archive session.

Background: tg-archiver uses forward-as-copy (no file downloads). Pause/resume
is cursor-based — the `last_forwarded_message_id` field in per-channel state
is updated atomically after each chunk, so a restart always resumes from the
correct position without byte-level tracking.

Expected behaviour:
- The user can pause a running archive from the progress view in the TUI.
  The current chunk is allowed to complete. No new chunks are started after
  the pause signal is received.
- Pausing triggers an immediate atomic state flush to
  `~/.local/state/tg-archiver/state-{channel_id}.json` (write to `.tmp`, rename).
- On resume (same session or new session), the app reads the channel's state
  file, finds `last_forwarded_message_id`, and starts the next chunk from
  `last_forwarded_message_id + 1`. No message is re-forwarded.
- On startup, if a channel's state has a `last_forwarded_message_id` that is
  less than the source channel's total message count, offer the user a TUI
  prompt to resume or start fresh. Starting fresh resets
  `last_forwarded_message_id` to `None` — it does not delete the state file.

Implementation rules:
1. The pause signal is an `AppEvent` variant sent from the TUI input handler
   to `App::handle_event()`. The worker reads a shared
   `tokio::sync::watch` or `AtomicBool` pause flag — do not cancel tasks,
   use the flag to gate new chunk fetching.
2. State flush on pause must use the atomic write pattern:
   `<path>.tmp` → rename via `tokio::fs`.
3. Do not use `unwrap()` or `expect()` anywhere in the pause/resume path.
4. Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build
   --release`, and `cargo test -- --test-threads=1` — all must exit 0.
