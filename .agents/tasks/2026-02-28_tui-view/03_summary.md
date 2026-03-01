# TUI Scaffold Summary

The application shell has been successfully implemented using `ratatui` with the `crossterm` backend.

## Visual Layout
The UI currently displays a full-screen, single block layout acting as a placeholder:
- **Border**: A thin blue line surrounding the entire terminal pane (`Borders::ALL`).
- **Title**: "tg-archiver" printed at the top-left edge of the border.
- **Content**: A paragraph block displaying:
  - "Welcome to tg-archiver shell."
  - "Configured api_id: [ID]"
  - "Pending downloads: [COUNT]"
  - "Press 'q' or Ctrl-C to quit."

## Architecture Additions
- **`App` Struct**: The single source of application state and configuration truth, found in `src/app/mod.rs`.
- **`AppEvent` Enum**: The domain-typed message channel (`AppEvent::Input` for key events, `AppEvent::Tick` for periodic ticks).
- **Asynchronous Loop**: An unblocking `mpsc` channel routes inputs from a dedicated `tokio::task::spawn_blocking` thread into the main async `run_app` loop within `src/main.rs`.

All required project rules have been adhered to, and all `crossterm` related state cleans itself up sequentially upon task exit.

### Refinements & Fixes
- **Robust Cleanup:** `disable_raw_mode()` and `LeaveAlternateScreen` are now structurally guaranteed to execute upon async `run_app()` completion via separated lifecycle handling inside `tokio::main`. Errors within the application loop will naturally propagate outward without stranding the user's terminal in an unusable state.
- **State & Access Control:** Internal struct variables inside `App` (`config`, `state`, `should_quit`) are now strictly marked as private fields with mapped getter methods. This prevents asynchronous TUI renderers or generic worker threads from mutating single-source-of-truth application data without explicitly channeling changes through the `AppEvent` message broker.
