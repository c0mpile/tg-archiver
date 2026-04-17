---
description: Add a Telegram API operation
---

Using the tg-archiver project rules, implement a new Telegram API operation.

The operation is: [DESCRIBE WHAT THE OPERATION DOES]

Rules to follow:
1. All Telegram API logic lives in `src/telegram/`. Do not call
   `grammers_client::Client` methods from any other module.
2. The operation must be implemented as a method on the `TelegramClient`
   newtype, not as a free function.
3. The method must be wrapped in the `retry_flood_wait!` macro already defined
   in `src/telegram/`. A call to any grammers method that bypasses this macro
   is a bug. The retry policy is: catch `FloodWait`, sleep for the indicated
   duration plus 2 seconds, retry exactly once, then propagate as
   `AppError::FloodWait` if it fails again.
4. If the operation resolves a channel or group by name or username, cache the
   result in the in-memory resolution cache in `src/telegram/`. Do not perform
   a fresh resolution on every call.
5. If the operation requires a raw TL call (i.e. grammers 0.9.0 does not
   expose it natively), use `self.client.invoke(&req)` with a
   `grammers_tl_types::functions::*` struct. Verify the function exists in
   the pinned `grammers-tl-types` version before use.
6. If the operation iterates messages, use `get_messages_by_id` with a chunk
   size of 100. Process and persist state after each chunk — do not accumulate
   all results before processing.
7. Return type must be `anyhow::Result<T>`. Attach context strings to all `?`
   calls using `.context("…")` so errors are traceable without a debugger.
8. Write a unit test where practical. If the operation requires a live client,
   gate the test with `#[ignore]` and add the comment
   `// requires: TG_API_ID, TG_API_HASH`.
9. Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build
   --release`, and `cargo test -- --test-threads=1` — all must exit 0.
