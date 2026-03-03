---
trigger: always_on
---

# tg-archiver — Tool & Framework Rules

## grammers-client

- Initialise the client **once** at startup inside `src/telegram/`. Pass a
  `TelegramClient` handle (wrapping `grammers_client::Client`) to the archive
  worker pool — do not re-initialise per task.
- All channel/group resolution (name → ID) must be cached in memory after the
  first lookup. Do not re-resolve on every message fetch.
- Use `client.iter_messages(chat)` for paginated message scanning. Set chunk
  size to 100 messages per request. Never fetch all messages into memory at
  once — process and persist state incrementally as each chunk is scanned.
- Uploading large files: use `client.upload_file()` with chunked streaming.
  Do not read the entire file into a `Vec<u8>` before uploading.
- **`FloodWait` is the most common failure mode.** Every call site that
  touches the Telegram API must be wrapped in the flood-wait retry helper in
  `src/telegram/`. A bare unwrapped API call anywhere outside that module is
  a bug.

---

## Persistent State (`~/.local/state/tg-archiver/state.json` and others)

- The state directory must be created with `tokio::fs::create_dir_all` at
  startup if it does not exist.
- **Atomic writes only:** write to `<path>.tmp` first, then `tokio::fs::rename` over the target.
- State schema changes must use `#[serde(default)]` on all new fields so that
  existing state files deserialise without error.
- If a state file fails to deserialise (corrupt or incompatible schema), warn
  the user via the TUI and offer to reset to clean state.

Persisted state (`State` struct) includes:
- `source_channel_id` (Option<i64>)
- `source_channel_title` (String)
- `dest_group_id` (Option<i64>)
- `dest_group_title` (String)
- `dest_topic_id` (Option<i32>)
- `dest_topic_title` (String)
- `auto_create_topic` (bool)
- `last_forwarded_message_id` (Option<i32>)
- `post_count_threshold` (u32)

Additionally:
- A `LastSession` struct and `last_session.json` pointer file exist to track the most recently active channel.
- Per-channel state is stored at `state-{id}.json`.

TUI-ephemeral state (cursor positions, selected indices, input field buffers)
lives on the `App` struct and is **not** written to disk.

---

## ratatui

- The TUI render loop runs on the tokio main thread. Blocking inside the
  render or input-handling path is prohibited — offload all async work to
  spawned tasks that send `AppEvent` messages back to the main loop.
- Minimum required views:
  - ChannelSelect (source selection with search)
  - GroupSelect (destination group selection)
  - TopicSelect (destination topic selection)
  - FilterConfig (configuration like post count threshold)
  - ArchiveProgress (scrollable log panel with ArchiveLogLine entries, auto-scroll, PageUp/PageDown navigation, completion state)
  - Log/error panel

---

## Raw TL API Calls

- `forward_messages_as_copy` and `create_topic` both use `self.client.invoke(&req)` with raw `grammers_tl_types::functions` structs.
- All raw TL invocations must be wrapped in `retry_flood_wait!`.
- New TL functions must be verified against the grammers 0.9.0 `grammers-tl-types` crate before use.

---

## dotenvy

- Load `.env` as the **very first action** in `main()`, before any other
  initialisation. If `dotenvy::dotenv()` returns an error because `.env` is
  missing, continue silently (variables may be set in the shell environment).
- If a required variable is absent after loading, call
  `std::process::exit(1)` with a message that names the missing key exactly.

---

## Pre-Completion Checklist

Before marking any task complete, verify all of the following in addition to
the global checklist:

- [ ] `cargo fmt -- --check` exits 0
- [ ] `cargo clippy -- -D warnings` exits 0
- [ ] `cargo build --release` exits 0
- [ ] `cargo test -- --test-threads=1` exits 0 (all non-`#[ignore]` tests pass)
- [ ] No `.env`, `*.session`, or credential values in staged files
- [ ] Any new persisted state fields use `#[serde(default)]`
- [ ] Any new Telegram API call site is wrapped in the flood-wait retry helper in `src/telegram/`
- [ ] Any new `async fn` doing file I/O uses `tokio::fs`, not `std::fs`
- [ ] State writes use the `.tmp` → rename atomic pattern
- [ ] `.env.example` updated if new required env variables were added