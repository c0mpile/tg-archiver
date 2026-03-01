# Implementation Plan: tg-archiver

This document breaks down the full implementation of the `tg-archiver` application into discrete, independently executable subtasks in adherence to the `tg-archiver-arch.md`, `tg-archiver-core.md`, and `tg-archiver-tools.md` rules.

## Subtask 1: Central Config and State Schema
- **Workflow**: `/state-schema-change`
- **Files Touched**: `src/config/mod.rs`, `src/state/mod.rs`, `src/app/mod.rs`
- **Description**: Define the `Config` struct initialized from environment variables. Define the persistent `State` struct matching the schema requirements (source ID, dest ID/topic, active filters, local path, message cursor, file download statuses). Implement the atomic state save/load lifecycle using the `state.json.tmp` -> `state.json` pattern via `tokio::fs`.
- **Dependencies**: None

## Subtask 2: TUI Scaffold and App Event Loop
- **Workflow**: `/tui-view`
- **Files Touched**: `src/main.rs`, `src/app/mod.rs`, `src/tui/mod.rs`
- **Description**: Set up `ratatui` with the `crossterm` backend. Create the top-level `App` struct as the single source of truth, and `AppEvent` enum (user input, render ticks, worker progress updates). Implement the asynchronous main loop that feeds events safely into `App::handle_event()`.
- **Dependencies**: Subtask 1

## Subtask 3: Telegram Client Auth & Flood-Wait Helper
- **Workflow**: `/add-telegram-operation`
- **Files Touched**: `src/telegram/mod.rs`, `src/error.rs`, `src/main.rs`
- **Description**: Wrap `grammers_client::Client` inside a new type `TelegramClient`. Implement interactive Telegram authentication logic (via standard input if the `TG_SESSION_FILE` is invalid/missing). Add the mandatory `FloodWait` retry wrapper helper for all subsequent Telegram API calls.
- **Dependencies**: Subtask 1

## Subtask 4: Channel & Group Resolution UIs
- **Workflow**: `/tui-view` (and `/add-telegram-operation`)
- **Files Touched**: `src/tui/mod.rs`, `src/telegram/mod.rs`, `src/app/mod.rs`
- **Description**: Implement TUI forms to input the source public channel and destination private group. Add Telegram lookup calls to resolve display names into canonical IDs. List and select the target topic within the target group. Persist resolved IDs into `State`.
- **Dependencies**: Subtask 2, Subtask 3

## Subtask 5: Filter Configuration UI
- **Workflow**: `/tui-view`
- **Files Touched**: `src/tui/mod.rs`, `src/state/mod.rs`, `src/app/mod.rs`
- **Description**: Implement TUI menus to toggle filtering config (file type categories: Video/Audio/Image/Archive, minimum file size, post count threshold) and the local download destination path. Update internal state and conditionally write to disk when confirmed.
- **Dependencies**: Subtask 2, Subtask 1

## Subtask 6: Archive Worker Pool and Content Scanner
- **Workflow**: `/new-feature`
- **Files Touched**: `src/archive/mod.rs`, `src/telegram/mod.rs`, `src/app/mod.rs`
- **Description**: Instantiate the bounded `tokio::sync::Semaphore`-gated worker pool (default 3 concurrent bounds). Implement paginated `iter_messages()` scanning. Identify media types based on filter state. Dispatch async download tasks that stream file chunks `into_bytes()` to the local filesystem without blocking memory thresholds. Build the TUI progress dashboard integration. 
- **Dependencies**: Subtask 3, Subtask 4, Subtask 5

## Subtask 7: Media Description Heuristic
- **Workflow**: `/add-telegram-operation`
- **Files Touched**: `src/telegram/mod.rs`, `src/archive/mod.rs`
- **Description**: Implement the fallback description lookup logic. If media message text is empty, fetch the preceding message ID. Save results as `.txt` files alongside the downloaded local media file during the worker pool download step.
- **Dependencies**: Subtask 6

## Subtask 8: Destination Topic Uploading
- **Workflow**: `/add-telegram-operation`
- **Files Touched**: `src/telegram/mod.rs`, `src/archive/mod.rs`
- **Description**: Implement chunked file uploading to the destination channel. Pair the uploaded media file with the scraped description text as the caption. Tie this upload step to the completion block of the file download worker.
- **Dependencies**: Subtask 6, Subtask 7

## Subtask 9: Pause/Resume Architecture & State Persistence
- **Workflow**: `/pause-resume`
- **Files Touched**: `src/archive/mod.rs`, `src/state/mod.rs`, `src/app/mod.rs`
- **Description**: Enhance the worker pool to safely record per-file statusing (`Pending`, `InProgress { bytes_received }`, `Complete`, `Failed`, `Skipped`) via `AppEvent` feedback loops. Load the message ID cursor from state at startup to immediately resume archive tracking without re-scanning previously analyzed logs. Include a robust exit handler.
- **Dependencies**: Subtask 8, Subtask 6
