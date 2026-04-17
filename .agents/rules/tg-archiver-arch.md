---
trigger: always_on
---

# tg-archiver — Architecture Rules

## App State Machine

The `App` struct in `src/app/` owns the entire application state and is the
single source of truth. The TUI event loop and the archive worker both
communicate with `App` exclusively through typed message enums (`AppEvent`).
Do not let TUI code or Telegram code mutate state directly — all mutations go
through `App::handle_event()`.

---

## Concurrency Model

The archive worker runs as a single sequential tokio task — **no parallel
downloads, no semaphore pool.** Forward-as-copy is rate-limited by Telegram's
flood-wait mechanism, not by I/O concurrency. The worker sends `AppEvent`
messages back to the main loop to update TUI state.

All Telegram API calls must respect `FloodWait` errors via the `retry_flood_wait!`
macro (see Telegram Client Rules below). A bare unwrapped API call anywhere
outside `src/telegram/` is a bug.

---

## Forward-as-Copy Worker

The archive worker lives in `src/archive/mod.rs`. Its operation is strictly sequential:

1. **Message range scan:** Fetch message IDs from the source channel using
   `get_messages_by_id` in chunks of **100 messages**, starting from
   `state.last_forwarded_message_id + 1`.
2. **Service message filtering:** Skip messages where
   `text().trim().is_empty() && media().is_none()`. This heuristic correctly
   covers Telegram service messages without brittle raw TL enum matching.
3. **Batch forwarding:** Forward non-service messages via
   `TelegramClient::forward_messages_as_copy`, which calls raw TL
   `messages.ForwardMessages` with `drop_author: true`.
4. **`random_id` generation:** **Critical** — must use `base_micros + i as i64`
   via `.enumerate()`. Never generate all IDs from the same `SystemTime::now()`
   call — all values will be identical within a single iterator pass and
   Telegram will silently drop all but the first message per duplicate ID.
   ```rust
   let base_micros = SystemTime::now()
       .duration_since(UNIX_EPOCH).unwrap().as_micros() as i64;
   let random_id: Vec<i64> = msg_ids
       .iter().enumerate()
       .map(|(i, _)| base_micros + i as i64)
       .collect();
   ```
5. **Cursor update:** After each chunk, update `state.last_forwarded_message_id`
   to `current_end` and save state atomically before processing the next chunk.
6. **Inter-chunk delay:** `tokio::time::sleep(Duration::from_millis(500))` between
   chunk requests. Never scan in a tight loop.

---

## Peer Cache Warm-up

Before `start_archive_run` is called, **both** `source_channel_id` and
`dest_group_id` must have entries in the in-memory peer cache in `src/telegram/`.
If either peer is absent from the cache, call `get_joined_channels()` or
`get_joined_groups()` respectively to warm the cache before spawning the worker.
An archive run started with a cold peer cache will fail when the first
`ForwardMessages` call tries to resolve the destination.

---

## Auto-Topic Creation

When the user selects "Create Automatically" in the destination topic picker:

- `state.auto_create_topic` is set to `true`; `state.dest_topic_id` is `None`
- In the `s` key handler, before validation, check `state.auto_create_topic`:
  - If `true`: call `telegram_client.create_topic(dest_group_id, &title).await`
  - Set `state.dest_topic_id = Some(returned_id)` and `state.auto_create_topic = false`
  - Save state atomically, then proceed to start the archive run
- Only after this step should a `None` `dest_topic_id` be treated as a validation error
- The topic title for auto-creation is derived from the source channel title

---

## Prohibited Patterns

- No synchronous file I/O (`std::fs`) inside `async` functions — use
  `tokio::fs` throughout.
- Do not call `unwrap()` or `expect()` anywhere in non-test code. All
  fallible paths must use `?` with `anyhow::Context` to attach context.
- Do not spawn `std::thread` threads for Telegram operations — all Telegram
  calls must run on the tokio runtime.
- Do not store the raw `grammers_client::Client` outside `src/telegram/` —
  wrap it in a `TelegramClient` newtype that owns retry and flood-wait logic.
- Do not add a wildcard `_` arm to paper over missing match cases — update
  every arm explicitly.
- Do not delete test files to keep `cargo test` green — fix the test or gate
  it with `#[ignore]`.

---

## Raw TL API Calls

Both `forward_messages_as_copy` and `create_topic` use raw TL invocation:

```rust
self.client.invoke(&req).await
```

where `req` is a `grammers_tl_types::functions::*` struct. Rules:

- All raw TL invocations **must** be wrapped in `retry_flood_wait!`
- Before adding a new raw TL call, verify the function exists in the
  `grammers-tl-types` crate at the version pinned in `Cargo.toml`
- `create_topic` parses the `CreateForumTopic` response by looking for
  `Update::NewMessage` or `Update::NewChannelMessage` wrapping a
  `Message::Service` — that service message's `id` is the topic ID.
  Falls back to `list_topics()` + title-match if the primary parse yields nothing.

---

## Telegram Client Rules

- The `retry_flood_wait!` macro (not a function) is the flood-wait wrapper.
  It **must** wrap every grammers API call — including raw TL `invoke` calls.
- Flood-wait policy: catch `FloodWait`, sleep for indicated duration + 2s buffer,
  retry exactly once, then propagate as `AppError::FloodWait`.
- All channel/group resolution (name → ID) must be cached in memory after the
  first lookup. Do not re-resolve on every call.
- The peer cache must be warm before starting any archive run (see Peer Cache
  Warm-up section above).
