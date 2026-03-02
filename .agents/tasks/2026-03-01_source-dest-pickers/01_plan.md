# 01 Plan: Source and Destination Pickers

## Approach
1. **TelegramClient enhancements**:
   - Add memory caches `channel_list_cache: Arc<RwLock<Option<Vec<(i64, String)>>>>` and `group_list_cache`.
   - Implement `get_joined_channels()` and `get_joined_groups()`. Both will use `client.iter_dialogs()`.
   - Wrap `iter_dialogs().next()` inside `retry_flood_wait!`.
   - Filter `grammers_client::types::Chat` for broadcasts (channels) and groups/megagroups.
   - Cache results in memory after the first fetch.
   - Also insert each fetched dialog's `InputPeer` into `peer_cache` so that topic fetching and message iteration works automatically.

2. **App State Machine (`App` / `AppEvent`)**:
   - Add fields for list state: `available_channels`, `selected_channel_index`, `available_groups`, `selected_group_index`.
   - Replace `ChannelResolved` / `GroupResolved` text input events with list navigation (`j`/`k`/`Up`/`Down`) events.
   - When entering the views from Home, trigger a Tokio spawn task that calls `get_joined_channels()` / `get_joined_groups()` and sends back `ChannelsLoaded` / `GroupsLoaded`.
   - Upon `Enter` in the picker, set the destination/source directly (no second API call needed), then trigger `TopicsLoaded` if a group is selected.

3. **TUI Rendering**:
   - Remove `render_input` from `tui/mod.rs` since text input is deleted.
   - Write list renderers for `ActiveView::ChannelSelect` and `ActiveView::GroupSelect` matching the `TopicSelect` style. Format the list element as `{id}  {title}`. Scroll will be native to ratatui's `ListState` or done manually via windowing if we don't use `ListState`. We'll use a manual window or rely on ratatui's `List` widget properties. Actually, `ratatui::widgets::List` supports `ListState` for scrolling. We'll add `ListState` or handle slicing manually. Currently `render_topic_select` doesn't seem to use `ListState`, it just renders all items. We will update it or slice `[skip..]` to handle long lists scrolling appropriately to terminal height constraint.

## Files to Modify
- `src/telegram/mod.rs`
- `src/app/mod.rs`
- `src/tui/mod.rs`

## Risks & Open Questions
- `iter_dialogs()` could take time if the user is in thousands of groups/channels. We'll fetch all into memory on the first request. The user must wait, but TUI will remain responsive because the fetch runs in a separate spawned task.
- Pagination in ratatui: Currently `TopicSelect` does not paginate, it might overflow the frame. For these new pickers, I will use `ratatui::widgets::List::new(...)` combined with a calculated window or `ListState` to ensure it scrolls properly ("If the list is long, the view must scroll").


### Feedback Addendum
- Update `TopicSelect` to use `ratatui::widgets::ListState` instead of slice/manual scrolling.
- Display a "Loading..." message in the picker view while channels/groups are being fetched.
- Ensure `ChannelSelect` and `GroupSelect` use `.id()` and `.name()` formatted as `{id}  {title}` and manage scroll using `ListState`.
