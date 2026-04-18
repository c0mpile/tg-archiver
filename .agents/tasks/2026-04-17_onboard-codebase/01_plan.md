# tg-archiver Onboarding Plan

## What the Project Does
`tg-archiver` is a Rust TUI application that archives Telegram channels by mirroring their contents into topics within a private group. It operates entirely server-side using Telegram's `messages.ForwardMessages` mechanism with `drop_author: true` (forward-as-copy). It does not download or upload media files directly.

## Module Layout
- `src/main.rs`: Application entry point. Initializes configuration, loads `.env`, sets up the XDG state directory, initializes the Telegram client (with blocking authentication if needed), and starts the TUI event loop.
- `src/app/`: Contains the `App` struct, which is the single source of truth for the application's state machine, and the definitions for the `AppEvent` message-passing system.
- `src/tui/`: Handles the terminal user interface rendering using `ratatui`.
- `src/telegram/`: Wraps `grammers-client`. Manages initialization, channel/group resolution caching, and handles the raw TL (Type Language) calls needed for forwarding messages and creating topics. Crucially, implements the `retry_flood_wait!` macro.
- `src/archive/`: Contains the forward-as-copy worker. Operates sequentially in a separate tokio task, scanning messages in chunks of 100, filtering service messages, and updating the cursor.
- `src/state/`: Manages persistent, per-channel application state using `serde_json`, with atomic file writes to `~/.local/state/tg-archiver/`.
- `src/config/`: Configuration struct and `.env` loading.
- `src/error.rs`: Unified error type definition (`AppError`) using `thiserror`.

## Key Data Flows
1. **Event Loop**: Input from `crossterm` and ticks are sent via `mpsc::channel` to the main render loop as `AppEvent`s. The `App::handle_event` method processes these to update ephemeral state or dispatch side effects.
2. **Archive Worker**: The worker task communicates with the main thread by sending `AppEvent::ArchiveLog`, `AppEvent::ArchiveComplete`, or `AppEvent::ArchiveError`.
3. **Telegram API**: Calls go through the `TelegramClient` wrapper. All API calls, including raw TL invocations like `forward_messages_as_copy` and `create_topic`, are wrapped in the `retry_flood_wait!` macro to gracefully handle rate-limits.
4. **State Persistence**: After processing each 100-message chunk, the cursor (`last_forwarded_message_id`) is updated and the state is persisted to disk using an atomic write pattern (`.tmp` followed by a rename).

## Obvious Technical Debt / Incomplete Areas
- **Testing Constraints**: Tests must be run with `--test-threads=1` because state tests mutate global environment variables (`std::env::set_var`).
- **grammers 0.9.0 Limitations**: Because `forward_messages` with `drop_author` and `create_topic` are not natively exposed in grammers `0.9.0`, raw TL function structs (`grammers_tl_types`) are heavily relied upon. Any API updates from Telegram might require careful version bumps.
- **No Parallelism**: To adhere strictly to Telegram's flood-wait limits, the architecture prohibits parallel processing. This is a deliberate design choice rather than debt, but it constraints throughput.
- **Absence of generic TODOs**: A sweep of the codebase revealed no lingering `TODO` or `FIXME` comments, which suggests the codebase is reasonably well-maintained or recently stabilized (indicated by a recent transition away from parallel file downloads).

## What I need to know to work confidently
- The project rules (`tg-archiver-arch.md`, `tg-archiver-core.md`, `tg-archiver-tools.md`) strictly dictate how state is managed, how the TUI is separated from the worker, and how Telegram's API must be queried.
- I must not introduce asynchronous file I/O using `std::fs` (must use `tokio::fs`).
- I must ensure `#[serde(default)]` is used for any new state fields.
- I must run `cargo clippy -- -D warnings`, `cargo fmt`, and `cargo test -- --test-threads=1` before considering any task complete.

## Open Questions
- Since tests relying on live credentials are gated with `#[ignore]`, should I attempt to build mocked tests for new features, or rely on manual validation in a sandbox environment?
- Are there specific naming conventions for branches/commits beyond the standard `feat(scope): ...` format?
