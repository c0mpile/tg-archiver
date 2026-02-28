---
description: Debug a flood-wait or rate-limit ban
---

Using the tg-archiver project rules, investigate and fix a flood-wait or
rate-limiting issue reported during an archive run.

Diagnostic steps:
1. Check `src/telegram/` for any call site that reaches a grammers method
   without going through the flood-wait retry helper. List every such site
   in `01_plan.md` before making any changes.
2. Check the concurrency semaphore limit in `src/archive/`. Confirm it is
   set to 3 by default and that no code path raises it above 5 without a
   user confirmation prompt.
3. Check whether any message scanning loop in `src/archive/` issues API calls
   in a tight loop without yielding between chunks. There must be at least a
   short `tokio::time::sleep` between chunk requests (minimum 500ms) when
   scanning large channels.
4. If the issue is a specific error variant from grammers (e.g.
   `RPCError { code: 420 }`), identify exactly which call site triggers it
   and confirm the flood-wait helper covers that error variant.

Fix rules:
- All fixes go in `src/telegram/` or `src/archive/`. Do not widen flood-wait
  handling with a blanket catch in `main()`.
- After any fix, run `cargo fmt -- --check`, `cargo clippy -- -D warnings`,
  `cargo build --release`, and `cargo test` — all must exit 0.
- Document the root cause and the fix in `03_summary.md`.