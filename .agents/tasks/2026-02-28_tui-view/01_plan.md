# TUI Scaffold and App Event Loop Plan

## Objective
Set up the ratatui + crossterm backend, implement the top-level `App` struct and async event loop as described in Subtask 2.

## Implementation Steps
1. **Define AppEvent and App (`src/app/mod.rs`)**
   - Create `AppEvent` enum with variants: `Input(crossterm::event::KeyEvent)`, `Tick`, `Quit`.
   - Create `App` struct to hold `config`, `state`, and `should_quit` flag.
   - Implement `App::new(config, state)` and `App::handle_event(&mut self, event: AppEvent)`.
   - Implement a basic `App::draw` method that clears the screen or just delegates to a placeholder view in `src/tui/mod.rs`.

2. **Define Base TUI Structure (`src/tui/mod.rs`)**
   - Implement a simple `render` function that takes `&mut ratatui::Frame` and `&App` and draws a basic skeleton (e.g. a placeholder or title bar) so there is a visual shell.

3. **Wire up Main Event Loop (`src/main.rs`)**
   - After config and state loading, authenticate Telegram before TUI init (this is a placeholder for now as per instructions "Telegram authentication must happen before the TUI initialises").
   - Initialize crossterm (`enable_raw_mode()`, `EnterAlternateScreen`).
   - Initialize ratatui `Terminal`.
   - Spawn an async task that listens to crossterm events (using `crossterm::event::EventStream` and `tokio_stream::StreamExt`) and sends them to an mpsc channel as `AppEvent::Input`.
   - Option: also send `AppEvent::Tick` periodically.
   - In the main loop:
     - `terminal.draw(|f| tui::render(f, &app))?;`
     - Wait for `AppEvent` from the channel.
     - Call `app.handle_event(event)`.
     - Exit loop if `app.should_quit`.
   - Clean up terminal (`disable_raw_mode()`, `LeaveAlternateScreen`).

4. **Verification**
   - Run `cargo fmt -- --check`.
   - Run `cargo clippy -- -D warnings`.
   - Run `cargo build --release`.
   - Manually verify the TUI runs and quits cleanly on 'q' or 'Ctrl-c', and is full-screen without glitching.
   - Write `02_terminal.log` and `03_summary.md`.
