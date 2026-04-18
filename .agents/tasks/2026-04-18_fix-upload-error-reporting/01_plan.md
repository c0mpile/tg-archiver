# Plan - Fix Upload Error Reporting

## Problem
In `src/upload/mod.rs`, the `upload_file` function uses `.context()` which doesn't always show the full underlying error chain in the way the TUI expects for `UploadWarning`.

## Proposed Change
Change:
```rust
.context("Failed to upload file to Telegram server")
```
to:
```rust
.map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))
```
in `src/upload/mod.rs`.

## Verification Plan
1. `cargo fmt -- --check`
2. `cargo clippy -- -D warnings`
3. `cargo build --release`
4. `cargo test -- --test-threads=1`
