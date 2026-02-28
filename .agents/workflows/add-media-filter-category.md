---
description: Add a new media type filter category
---

Using the tg-archiver project rules, add support for a new filterable media
category.

1. Add the new variant to the filter category enum in `src/config/`.
2. Map the new category to its grammers `Media` variant(s) and MIME types in
   the filtering logic in `src/archive/`, following the existing pattern for
   Video/Audio/Image/Archive. MIME matching must use prefix matching for
   wildcard types (e.g. `video/*`) and exact matching for specific types.
3. Add the new category as a toggleable option in the filter configuration
   TUI view in `src/tui/`, consistent with the existing checkbox-style
   multi-select pattern.
4. Add the new variant to the serde-serialised filter config in `src/state/`
   with `#[serde(default)]` so existing state files deserialise without error.
5. Write a unit test in `src/archive/` that asserts a message carrying the
   new media type is accepted when the category is enabled and rejected when
   it is disabled.
6. Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build
   --release`, and `cargo test` — all must exit 0 before marking complete.