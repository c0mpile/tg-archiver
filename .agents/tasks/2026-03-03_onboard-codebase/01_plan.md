# Onboard Codebase Plan

## Project Overview
`tg-archiver` is a terminal application driven by `ratatui` that mirrors media files from a public Telegram source channel to a topic inside a private destination group, with parallel downloads, configurable filters, and full pause/resume support across sessions.

## Module Layout
- `src/main.rs`: Entry point, `.env` loading, async runtime initialization.
- `src/app/`: Top-level `App` struct, state machine, and main TUI event loop.
- `src/tui/`: `ratatui` widgets and views (ChannelSelect, GroupSelect, TopicSelect, FilterConfig, ArchiveProgress).
- `src/telegram/`: `grammers-client` wrappers, raw MTProto API calls (e.g., `forward_messages_as_copy`, `create_topic`), and flood-wait retry macro.
- `src/archive/`: Forward-as-copy worker pool, chunked message scanning, and cursor management.
- `src/state/`: Persistent state (`state.json` and per-channel state), utilizing `serde` with atomic renaming.
- `src/config/`: Configuration struct and `.env` loading validation.
- `src/error.rs`: Unified error types using `thiserror` and `anyhow`.

## Key Data Flows
1. **Application State**: The `App` struct is the single source of truth. All events (TUI input, Telegram events) are routed through `AppEvent` and processed in `App::handle_event()`.
2. **Forwarding Pipeline**: The `archive` module manages a pool of workers gated by a `tokio::sync::Semaphore`. It fetches chunks of 100 messages via `get_messages_by_id`, filters service messages, and uses `forward_messages_as_copy` to mirror messages. Random IDs are generated sequentially to prevent duplicates.
3. **Peer Cache**: Source and destination peers are resolved and cached prior to initiating an archive run to decouple network resolution from the forwarding loop.
4. **State Persistence**: Crucial operational state (cursors, thresholds, channel IDs) is saved via atomic tmp file replacement to avoid corruption.

## Obvious Technical Debt
1. The `retry_flood_wait!` macro is coupled to `Sender<AppEvent>` for logging, which should be refactored to just return the wait duration.
2. State tests currently use `unsafe { std::env::set_var }`, causing race conditions when tests run concurrently (hence the need for `cargo test -- --test-threads=1`).

## Requirements for Confident Work
- Adherence to the Surgical Modification philosophy (minimal diff noise, no unsolicited refactoring).
- Understanding of `grammers-client` primitives and raw TL API structures.
- Familiarity with the `tokio` asynchronous runtime, specifically avoiding blocking calls in the TUI thread.

## Open Questions
- None at this time. The architecture and rules have been thoroughly reviewed and updated to reflect the current state.
