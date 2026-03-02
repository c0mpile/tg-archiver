# Task Summary: Source/Dest Pickers

## Changes
- Modified `TelegramClient` in `src/telegram/mod.rs` to add `get_joined_channels()` and `get_joined_groups()`. These methods fetch and cache `Chat::Channel` and `Chat::Group` dialogs based on `broadcast` and `megagroup` flags, resolving them dynamically and storing their ID/Title and populated `InputPeer`s into `peer_cache`.
- Modified `App` state in `src/app/mod.rs` to store available channels / groups, as well as separate boolean flags for the loading states. We switched index tracking out for `ratatui::widgets::ListState`. Let the TUI display a "Loading..." message when asynchronously fetching dialogs.
- Overhauled `render_input` in `src/tui/mod.rs`, transforming the selection into beautiful text Pickers similar to `render_topic_select` using `ListState` and matching on `<ID> <Title>` text representations of Telegram chats.

## Verification
- Passed `cargo fmt -- --check`.
- Passed `cargo clippy -- -D warnings`.
- Passed `cargo test`.
- Passed `cargo build --release`.
- Built successfully and all formatting is strictly preserved. No rules broken.
