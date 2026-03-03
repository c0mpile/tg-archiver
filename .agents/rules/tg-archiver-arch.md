---
trigger: always_on
---

# tg-archiver — Architecture Rules

## App State Machine

The `App` struct in `src/app/` owns the entire application state and is the
single source of truth. The TUI event loop and the archive worker pool both
communicate with `App` exclusively through typed message enums (`AppEvent`).
Do not let TUI code or Telegram code mutate state directly — all mutations go
through `App::handle_event()`.

---

## Concurrency Model

Use `tokio` as the async runtime. The download worker pool lives in
`src/archive/` and is implemented as a bounded `tokio::sync::Semaphore`-gated
set of tasks.

- Default concurrency: **3 concurrent downloads**.
- Hard ceiling: **5 concurrent downloads**. Do not allow the user to set a
  higher value without an explicit confirmation prompt.
- All Telegram API calls must respect `FloodWait` errors: catch
  `grammers_client::types::errors::FloodWait`, extract the wait duration,
  sleep for that duration plus a 2-second buffer, then retry the failed call
  exactly once before propagating as a typed `AppError::FloodWait`.

---

## Prohibited Patterns

- No synchronous file I/O (`std::fs`) inside `async` functions — use
  `tokio::fs` throughout.
- Do not call `unwrap()` or `expect()` anywhere in non-test code. All
  fallible paths must use `?` with `anyhow::Context` to attach context.
- Do not spawn `std::thread` threads for Telegram operations — all Telegram
  calls must run on the tokio runtime.
- Do not store the raw `grammers_client::Client` outside `src/telegram/` —
  wrap it in a `TelegramClient` newtype that owns retry and flood-wait logic.

---

## Forward-as-Copy Worker

- Chunk size is 100 messages via `get_messages_by_id`.
- Service message skip heuristic: `text().trim().is_empty() && media().is_none()`.
- `random_id` must be `base_micros + i as i64` via enumerate — never duplicated within a batch.
- 500ms inter-chunk delay.
- Cursor updated to `current_end` after each chunk.
- `auto_create_topic: bool` on State triggers `create_topic()` before the run starts and is reset to `false` after.

---

## Peer Cache

Both source and dest peers must be in the peer cache before `start_archive_run` is called.

---

## Known Technical Debt

1. `retry_flood_wait!` macro accepts an optional `Sender<AppEvent>` to emit flood-wait log lines — this couples the telegram layer to app events and should be refactored to return the wait duration at the call site before monitoring mode is built.
2. State tests use `unsafe { std::env::set_var }` causing races under parallel test execution — run `cargo test -- --test-threads=1` until this is fixed.