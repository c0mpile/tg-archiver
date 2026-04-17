---
description: Debug a flood-wait or rate-limit ban
---

Using the tg-archiver project rules, investigate and fix a flood-wait or
rate-limiting issue reported during an archive run.

Diagnostic steps:
1. Check `src/telegram/` for any call site that reaches a grammers method
   without going through the `retry_flood_wait!` macro. List every such site
   in `01_plan.md` before making any changes.
2. Check whether any message scanning loop in `src/archive/` issues API calls
   in a tight loop without yielding between chunks. There must be at least a
   `tokio::time::sleep(Duration::from_millis(500))` between chunk requests
   when scanning large channels.
3. Check whether `random_id` generation in `forward_messages_as_copy` uses
   `base_micros + i as i64` via `.enumerate()`. If it uses a single
   `SystemTime::now()` call inside a tight `.map()`, IDs will collide and
   Telegram may reject the batch.
4. If the issue is a specific error variant from grammers (e.g.
   `RPCError { code: 420 }`), identify exactly which call site triggers it
   and confirm the `retry_flood_wait!` macro covers that error variant.

Fix rules:
- All fixes go in `src/telegram/` or `src/archive/`. Do not widen flood-wait
  handling with a blanket catch in `main()`.
- After any fix, run `cargo fmt -- --check`, `cargo clippy -- -D warnings`,
  `cargo build --release`, and `cargo test -- --test-threads=1` — all must exit 0.
- Document the root cause and the fix in `03_summary.md`.
