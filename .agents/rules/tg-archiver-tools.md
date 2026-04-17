---
trigger: always_on
---

# tg-archiver — Tool & Framework Rules

## grammers-client

- Initialise the client **once** at startup inside `src/telegram/`. Pass a
  `TelegramClient` handle (wrapping `grammers_client::Client`) to the archive
  worker — do not re-initialise per task.
- All channel/group resolution (name → ID) must be cached in memory after the
  first lookup. Do not re-resolve on every message fetch.
- Use `get_messages_by_id` for paginated message scanning. Set chunk size to
  100 messages per request. Process and persist state after each chunk —
  never accumulate all results before processing.
- **`FloodWait` is the most common failure mode.** Every call site that
  touches the Telegram API must be wrapped in the `retry_flood_wait!` macro
  in `src/telegram/`. A bare unwrapped API call anywhere outside that module
  is a bug.
- `Media::Video` does not exist in grammers 0.9.0. All video content is
  `Media::Document` with a `video/*` MIME type.

---

## Raw TL API Calls

`forward_messages_as_copy` and `create_topic` both require raw TL invocation
because grammers 0.9.0 does not expose these operations natively:

- Use `self.client.invoke(&req)` with `grammers_tl_types::functions::*` structs
- All raw TL invocations must be wrapped in `retry_flood_wait!`
- New TL functions added in future must be verified against the `grammers-tl-types`
  crate at the version pinned in `Cargo.toml` before use — do not assume a
  function exists without checking

---

## Persistent State

State is stored per-channel at `~/.local/state/tg-archiver/state-{channel_id}.json`.
A `~/.local/state/tg-archiver/last_session.json` pointer file records the last
active channel ID so the app can auto-load the correct state file on startup.

- The state directory must be created with `tokio::fs::create_dir_all` at
  startup if it does not exist.
- **Atomic writes only:** write to `<path>.tmp` first, then `tokio::fs::rename`
  over the target file. Never write directly to the state file in-place.
- State schema changes must use `#[serde(default)]` on all new fields so that
  existing state files deserialise without error.
- If a state file fails to deserialise (corrupt or incompatible schema), warn
  the user via the TUI and offer to reset to clean state. Never panic or
  silently overwrite without user confirmation.

### Current persisted state fields (per-channel `State` struct)

| Field | Type | Purpose |
|---|---|---|
| `source_channel_id` | `Option<i64>` | Resolved source channel ID |
| `source_channel_title` | `Option<String>` | Source channel display name |
| `source_message_count` | `Option<i32>` | Total message count in source (for progress %) |
| `dest_group_id` | `Option<i64>` | Resolved destination group ID |
| `dest_group_title` | `Option<String>` | Destination group display name |
| `dest_topic_id` | `Option<i32>` | Destination topic ID |
| `dest_topic_title` | `Option<String>` | Destination topic display name |
| `last_forwarded_message_id` | `Option<i32>` | Cursor — highest message ID already forwarded |
| `post_count_threshold` | `u32` | Max messages to forward per run (0 = no limit) |
| `auto_create_topic` | `bool` | If true, create a new topic before starting the run |

TUI-ephemeral state (cursor positions, selected indices, input field buffers,
`channel_loading` flag) lives on the `App` struct and is **not** written to state files.

### `last_session.json` format

```json
{ "last_channel_id": 1234567890 }
```

Written atomically (`.tmp` → rename) whenever the active channel changes.

---

## ratatui

- The TUI render loop runs on the tokio main thread. Blocking inside the
  render or input-handling path is prohibited — offload all async work to
  spawned tasks that send `AppEvent` messages back to the main loop.
- New user-initiated actions must be modelled as `AppEvent` variants handled
  in `App::handle_event()`, never handled directly in TUI render/input code.
- View state (cursor position, selected index, input buffer) is ephemeral —
  store on `App`, never persist to state files.

### Minimum required views

- Source channel selection (channel picker with search)
- Destination selection (group picker → topic picker, including "Create Automatically" option)
- Archive progress — verbose scrollable console log view:
  - Timestamped log lines per chunk: `[HH:MM:SS] Forwarded N messages (IDs X–Y)`
  - Flood-wait notifications when they occur
  - Status bar: `Forwarded up to ID: X / Y [Z%]`
  - Stays on completion screen until `q` (quit) or `r` (restart/new channel) is pressed
- Log/error panel

### `AppEvent` variants (key subset)

| Variant | Purpose |
|---|---|
| `ArchiveStarted` | Worker has begun; switches TUI to progress view |
| `ArchiveLog(String)` | Timestamped progress line from the worker |
| `ArchiveComplete` | Worker finished; progress view shows completion state |
| `ArchiveError(String)` | Worker encountered a fatal error |

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
- [ ] Any new Telegram API call site is wrapped in `retry_flood_wait!` in `src/telegram/`
- [ ] Any new `async fn` doing file I/O uses `tokio::fs`, not `std::fs`
- [ ] State writes use the `.tmp` → rename atomic pattern
- [ ] `.env.example` updated if new required env variables were added
- [ ] `rg '"[Auto-create topic:'` returns zero results (magic-string sentinel fully removed)
