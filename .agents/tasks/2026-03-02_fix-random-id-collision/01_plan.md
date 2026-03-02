# Task: Fix random_id collision in forward_messages_as_copy

The `forward_messages_as_copy` function in `repo/src/telegram/mod.rs` generates `random_id` values using `SystemTime::now().as_micros()` inside a tight loop. This causes duplicate IDs for large batches of messages, leading Telegram to drop duplicates for deduplication.

## Proposed Fix

1. Bind `base_micros` once before the iterator.
2. Use `.iter().enumerate().map(|(i, _)| base_micros + i as i64)` to ensure uniqueness within the batch.

## Checklist

- [ ] Apply surgical fix to `repo/src/telegram/mod.rs`
- [ ] Run `cargo fmt -- --check`
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Run `cargo build --release`
- [ ] Run `cargo test`
