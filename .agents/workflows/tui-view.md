---
description: Implement or modify a TUI view
---

Using the tg-archiver project rules, implement or modify a TUI view.

The target view is: [DESCRIBE THE VIEW OR CHANGE]

Rules to follow:
- All views live in `src/tui/`. Each distinct view or modal is its own module.
- The render loop runs on the tokio main thread. No blocking calls are
  permitted inside any render or input-handling function. Any async work must
  be offloaded to a spawned tokio task that sends an `AppEvent` back to
  `App::handle_event()`.
- View state (cursor position, selected index, input buffer contents) is
  ephemeral — store it on the `App` struct, never write it to
  `~/.local/state/tg-archiver/state.json`.
- New user-initiated actions (button presses, selections, confirmations) must
  be modelled as new `AppEvent` variants and handled in `App::handle_event()`,
  not handled directly inside the TUI render or input code.
- Follow the visual and interaction conventions already established in adjacent
  views (navigation keys, selection highlighting, error display placement).

After implementation:
- Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build
  --release` — all must exit 0.
- Visually describe the rendered layout in `03_summary.md` so it can be
  reviewed without running the binary.