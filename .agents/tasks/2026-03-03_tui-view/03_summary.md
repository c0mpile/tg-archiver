# Task Summary: Archive Progress TUI View

The Archive Progress screen has been transformed into a verbose, scrollable log panel that provides real-time status visibility during a forwarding run.

- **State Enhancements**: Introduced `ArchiveProgressState` tracking timestamped log messages, start boundaries, scroll offset, and completion state. Added new events: `AppEvent::ArchiveLog` and `AppEvent::ArchiveStarted`.
- **Worker Logging**: `src/archive/mod.rs` now emits rich log messages tracking chunks forwarded, chunks skipped, and completion status. `retry_flood_wait!` was enhanced to emit timed sleep logs back to the progress view.
- **TUI Update**: Replaced static state tracking with a `Paragraph` rendering logs bounded to a designated area. Integrated auto-scrolling with `Up/Down` and `PageUp/PageDown` navigation.
- **Dynamic Footer**: The status footer now displays a percentage layout block tracking overall progression. Upon completion, the footer clearly announces the run is complete and tells the user to press `q` or `r`.

All changes satisfy formatting, clippy rules, build, and tests natively.
