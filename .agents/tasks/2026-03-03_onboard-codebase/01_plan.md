# Onboarding Plan

## What the project does
`tg-archiver` is a terminal application that mirrors messages from a public Telegram source channel to a topic inside a private destination group using a forward-as-copy mechanism. It supports parallel and chunked downloads, pause/resume across sessions, and maintains per-channel state.

## Module Layout
- `src/main.rs`: Entry point, initializes runtime, TUI, and app state.
- `src/app/mod.rs`: Top-level `App` struct, event loop, state machine. Manages view switching and delegates TUI tasks.
- `src/state/mod.rs`: Persistent state serialization using `serde_json` and XDG directories, including per-channel states and a `last_session.json` pointer.
- `src/telegram/mod.rs`: `grammers-client` wrappers, raw TL API calls (e.g., `ForwardMessages`, `CreateForumTopic`), peer/topic caching, and the `retry_flood_wait!` macro.
- `src/archive/mod.rs`: Forward-as-copy worker, chunked message scanning (100 messages at a time via `get_messages_by_id`), cursor management, and pause/resume logic.
- `src/tui/`: Ratatui widgets, including `archive_progress.rs` which features a scrollable log panel and auto-scroll behavior.
- `src/config/`: Configuration structure and `.env` loading.
- `src/error.rs`: Unified application error types using `thiserror` and `anyhow`.

## Key Data Flows
1. **Startup**: Loads `.env`, initializes Telegram client (with auth if needed), reads `last_session.json`. If a session exists and has partial progress, jumps to `ResumePrompt` view, otherwise proceeds to channel selection.
2. **Channel Selection**: User selects source channel. State for that channel is loaded (`state-{id}.json`).
3. **Destination Selection**: User configures destination group and topic (including auto-create topic flag).
4. **Archive Run**: `start_archive_run` spawns a loop that queries `get_messages_by_id` in chunks of 100, filters out service/empty messages, and uses `forward_messages_as_copy` to transfer messages. Progress, cursor positions, and logs are communicated back to the `App` event loop via MPCS channels (`AppEvent`), and the `ArchiveProgress` view updates in real-time.

## Obvious Technical Debt / Incomplete Areas
1. `retry_flood_wait!` macro is coupled to `mpsc::Sender<AppEvent>`, sending log directly to the TUI. This breaks strict layering and should be refactored to return the wait duration at the call site before building monitoring modes.
2. State tests rely on `unsafe { std::env::set_var }` for the XDG directory, causing races during parallel execution (`cargo test -- --test-threads=1` is required for now).

## Open Questions
- Is there any plan to support media-only filtering again, or should all filtering logic remain removed?
- Should the `FloodWait` retry logic support exponential backoff, or is a flat wait + 2s buffer sufficient for all future use cases?
