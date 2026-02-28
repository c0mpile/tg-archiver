---
description: Add or change a persisted state field
---

Using the tg-archiver project rules, add, remove, or modify a field in the
persistent state schema.

The change is: [DESCRIBE THE FIELD CHANGE]

Rules to follow:
1. All state structs live in `src/state/`. Make the schema change there.
2. Every new field must carry `#[serde(default)]` so that existing
   `~/.local/state/tg-archiver/state.json` files deserialise without error.
   If a field is being removed, verify no other module references it before
   deletion.
3. If the change affects the per-file download status enum
   (`Pending | InProgress | Complete | Failed | Skipped`), update every
   `match` arm that exhaustively patterns on it throughout the codebase — do
   not add a wildcard `_` arm to paper over missing cases.
4. The atomic write path must remain intact: state is always written to
   `~/.local/state/tg-archiver/state.json.tmp` via `tokio::fs`, then renamed
   over `state.json`. Do not introduce any direct write to `state.json`.
5. If the state directory (`~/.local/state/tg-archiver/`) creation call in
   `main()` is touched, confirm it still uses `tokio::fs::create_dir_all`.
6. Write or update a unit test in `src/state/` that round-trips the modified
   struct through `serde_json::to_string` / `serde_json::from_str` and
   asserts field values survive serialisation.
7. Write a migration compatibility test: deserialise a hardcoded JSON string
   that represents the *previous* schema (without the new field) and assert
   that deserialisation succeeds and the new field takes its default value.
8. Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build
   --release`, and `cargo test` — all must exit 0.