# Plan: Fix Startup Memory Bug

The app currently starts blank on every launch because we moved to per-channel state files but didn't implement a "last session" pointer.

## Proposed Changes

### 1. `src/state/mod.rs`
- Add `LastSession` struct:
  ```rust
  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
  pub struct LastSession {
      #[serde(default)]
      pub last_channel_id: Option<i64>,
  }
  ```
- Implement `LastSession::save()` and `LastSession::load()`.
- Use `State::get_state_dir().join("last_session.json")`.
- Atomic writes for `last_session.json`.

### 2. `src/main.rs`
- In `main()`, after deriving `state_dir`:
- Try to load `LastSession`.
- If `last_channel_id` is found, call `State::load_for_channel(id)`.
- If state loaded has `last_forwarded_message_id.is_some()`, set `app.active_view = ActiveView::ResumePrompt`.

### 3. `src/app/mod.rs`
- Update `AppEvent::ChannelStateLoaded` handler to write `LastSession` with the new channel ID.

## Verification
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo build --release`
- `cargo test`
