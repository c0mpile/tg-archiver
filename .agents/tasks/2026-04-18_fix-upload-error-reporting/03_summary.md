# Summary - Fix Upload Error Reporting

## Changes
- Modified `src/upload/mod.rs` to replace `.context("Failed to upload file to Telegram server")` with `.map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))`.
- This ensures that the full error chain (root cause) is included in the error message, which is then passed to `AppEvent::UploadWarning` and displayed in the TUI.

## Verification Results
- `cargo fmt -- --check`: PASS
- `cargo clippy -- -D warnings`: PASS
- `cargo build --release`: PASS
- `cargo test -- --test-threads=1`: PASS
