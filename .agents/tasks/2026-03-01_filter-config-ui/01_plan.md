# Filter Configuration UI Plan

1. **State modification:** Add `include_text_descriptions` to `Filters` struct in `src/state/mod.rs` (defaulting to true) and update default tests. Note that we will also rely on `#serde(default)` for backwards compatibility.
2. **UI Data Types:** Create `FilterConfigField` enum to track currently focused field and `FilterConfigState` struct in `src/app/mod.rs` for tracking ephemeral input and editing values. Add `filter_config_state` field to `App`.
3. **App Event Setup:** Add `ActiveView::FilterConfig` and add a new menu option in `ActiveView::Home` mapped to `'3'`.
4. **View Logic:** Map inputs on the Filter Configuration UI natively inside `App::handle_event` on the `Input` branch. Support navigating up/down with arrow keys / `j`/`k`, toggling values, entering string mode for inputs like max counts or sizes, handling backspace/typing, and pressing "Enter" to finish editing or Save and exit the screen. Upon exit, serialize config to JSON by cloning App state and spawning a task.
5. **View Rendering:** Add `src/tui/filter_config.rs` which reads from `app.filter_config_state` to render a highlighted list widget with dynamic suffixes based on state or cursor editing status. Embed view in `tui::render`.
6. Verify code compiles, format is correct, and all linters pass.
