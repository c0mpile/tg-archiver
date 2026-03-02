# Task Summary: Fix random_id collision in forward_messages_as_copy

## Root Cause
In `repo/src/telegram/mod.rs`, the `forward_messages_as_copy` function was generating `random_id` values within a synchronous `.map()` loop using `SystemTime::now().as_micros()`. Because the loop executes within a few nanoseconds on modern hardware, most iterations returned the same microsecond value. Telegram's `ForwardMessages` API uses these `random_id` values for idempotency/deduplication, causing it to drop all but one message when multiple messages share the same ID.

## Fix
The solution was to bind a base timestamp once outside the loop and increment it for each message in the batch:

```rust
let base_micros = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_micros() as i64;
let random_id: Vec<i64> = msg_ids
    .iter()
    .enumerate()
    .map(|(i, _)| base_micros + i as i64)
    .collect();
```

## Verification Results
- [x] Applied surgical fix to `repo/src/telegram/mod.rs`
- [x] Build and tests passed (`cargo check`, `cargo fmt`, `cargo clippy`, `cargo build --release`, `cargo test`)
- [x] Zero unrelated changes or diff noise
