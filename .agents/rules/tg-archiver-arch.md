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

## File Type Filtering

Supported categories and their grammers media variant mappings:

| Category | grammers type |
|---|---|
| Video | `Media::Document` with MIME `video/*`, or `Media::Video` |
| Audio | `Media::Document` with MIME `audio/*`, or `Media::Audio` |
| Image | `Media::Photo`, or `Media::Document` with MIME `image/*` |
| Archive | `Media::Document` with MIME `application/zip`, `application/x-rar-compressed`, `application/x-7z-compressed`, `application/gzip`, `application/x-tar` |

The user must be able to select any combination of these four categories.
Filtering happens at message-scan time, before any download is attempted.

---

## Minimum File Size Threshold

Applied to `Document` types only (photos and audio do not expose byte size
reliably via grammers). Compare `document.size()` against the user-configured
minimum. Skip the file silently — record status as `Skipped`, not `Failed` —
if below threshold.

---

## Description Heuristic

When the "include text descriptions" option is enabled, apply this logic per
media file in the source channel:

1. If the same message that contains the media file also contains non-empty
   message text → that text is the description.
2. Otherwise, if the message immediately preceding the media message (by
   message ID) contains only text (no media) → that text is the description.
3. Otherwise → no description.

Descriptions are saved to disk as `<filename-without-extension>.txt` alongside
the media file in the local download destination. Descriptions are prepended
to the upload caption when posting to the destination topic.