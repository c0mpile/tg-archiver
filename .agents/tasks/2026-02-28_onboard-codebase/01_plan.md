# tg-archiver Codebase Onboarding Plan
Date: 2026-02-28

## What the Project Does
`tg-archiver` is a terminal application built in Rust that mirrors media files (video, audio, image, archive types) from a public Telegram source channel to a topic inside a private destination group. It supports parallel and configurable concurrent downloads (default 3, max 5), resuming interrupted downloads across sessions, and customizable filering for categories, size, and post counts.

## Module Layout
Currently, the source code directory (`repo/`) does not exist; the project is greenfield. Once created, the architecture rules dictate the following layout:
- `repo/src/main.rs`: Entry point.
- `repo/src/app/`: The central `App` struct representing the application state and event loop.
- `repo/src/tui/`: `ratatui` widgets and UI input handling. 
- `repo/src/telegram/`: `grammers-client` wrappers and flood-wait retry handling.
- `repo/src/archive/`: The async worker pool for parallel downloading and scheduling.
- `repo/src/config/`: Loading configurations from `.env`.
- `repo/src/state/`: JSON serialisation to disk at `~/.local/state/tg-archiver/state.json`.
- `repo/src/error.rs`: Centralized `anyhow` and `thiserror` types.

## Key Data Flows
- **Strict One-Way State Modification**: Both the TUI and the background tasks communicate with the central `App` state struct via a typed message enum (`AppEvent`). The TUI and background processes must never directly mutate the state.
- **Async Threading**: `tokio` backend. File downloads are gated by a tokio `Semaphore` up to the max concurrent limit. 

## Technical Debt / Incomplete Areas
- **Codebase is Missing**: The entire Rust codebase has not yet been initialized.
- **Workflow / Initial Setup Needs Attention**: Setup will require creating `.env` files, `.gitignore`, and the `repo/` folder alongside initial `cargo init`.

## Open Questions
1. Shall I go ahead and initialize the standard Rust scaffold in `repo/` (e.g., `cargo init repo`), install the required crates, and write the initial boilerplate outlined in the documentation?
2. Is there an existing `tg-archiver.session` file, or will we be handling the `grammers` authorization flow interactively next?
