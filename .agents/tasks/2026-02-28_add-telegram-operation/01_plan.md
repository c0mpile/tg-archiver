# 01_plan.md

1. Define `AppError` variants for Telegram auth and flood wait in `src/error.rs`.
2. Implement `TelegramClient` newtype wrapping `grammers_client::Client` in `src/telegram/mod.rs` using `grammers_client` v0.9 API.
3. Implement `retry_flood_wait` macro as a helper for future API calls.
4. Implement interactive Telegram auth logic (read session, fallback to terminal prompt for phone/code) in `src/telegram/mod.rs`.
5. Call the auth logic in `src/main.rs` before TUI initialization.
6. Run `cargo check`, `cargo clippy`, and `cargo test` to verify.
