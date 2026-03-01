# Plan: Channel & Group Resolution UIs

## 1. Telegram API (`src/telegram/mod.rs`)

Implement the following async functions on `TelegramClient`:

- `resolve_channel(username: &str) -> Result<(i64, String)>`:
  - Resolves a public channel given its username.
  - Caches the `(id, title)` internally after the first successful lookup.
  - Wrapped in `retry_flood_wait!`.
- `resolve_group(username: &str) -> Result<(i64, String)>`:
  - Resolves a private/public group given its username.
  - Caches results internally.
  - Wrapped in `retry_flood_wait!`.
- `list_topics(group_id: i64) -> Result<Vec<(i32, String)>>`:
  - Fetches the list of forum topics in a given group.
  - Caches the results.
  - Wrapped in `retry_flood_wait!`.

State structures to add to `TelegramClient`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// Add generic cache caches:
channel_cache: Arc<RwLock<HashMap<String, (i64, String)>>>,
group_cache: Arc<RwLock<HashMap<String, (i64, String)>>>,
topic_cache: Arc<RwLock<HashMap<i64, Vec<(i32, String)>>>>,
```

## 2. App Events (`src/app/mod.rs`)

Add standard events to process async lookup results and pass to the TUI.

```rust
pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    // Add new ones:
    ChannelResolved(Result<(i64, String), String>), // String is the error message to display
    GroupResolved(Result<(i64, String), String>),
    TopicsLoaded(Result<Vec<(i32, String)>, String>),
}
```

## 3. TUI Views (`src/tui/mod.rs` and modules)

We will manage the ephemeral state inside `App`, not `State`.
We need an enum to track which screen we are on, along with buffers for inputs and error messages:

```rust
pub enum ActiveView {
    Home,
    ChannelSelect,
    GroupSelect,
    TopicSelect,
}
```

For error surfacing:
- We will store a `resolution_error: Option<String>` in `App` ephemeral state.
- When an `AppEvent::ChannelResolved(Err(err))` occurs, we set `resolution_error = Some(err)` and do *not* mutate anything in `state.json`.
- The `ChannelSelect` or `GroupSelect` view will conditionally render this error message in bold red below the input box.
- Once the user types again, we can clear the `resolution_error`.

## 4. Updates to event handling

`handle_event` will respond to input. When the user hits Enter (submits a username), we start a tokio task that invokes the `TelegramClient` and publishes a `ChannelResolved` event back to the main loop. Upon successful resolution, we write to `state.json` via `app.state.save()`. Upon error, we *only* update ephemeral view state, ensuring `state.json` is untouched.

## 5. Verification

Run `cargo test`, `cargo fmt`, `cargo clippy`.
