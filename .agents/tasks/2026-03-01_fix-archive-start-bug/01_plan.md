# Plan: Fix Archive Start Bug

## Root Cause
When the archive run starts in `start_archive_run`, it attempts to download files to `state.local_download_path`. However, if this directory does not exist, `tokio::fs::File::create` fails with "No such file or directory (os error 2)". There is no code that ensures the download directory exists before the archive workers begin downloading files.

## Proposed Fix
1. Modify `src/archive/mod.rs` inside the `tokio::spawn` block of `start_archive_run`.
2. Before calling `run_archive_loop`, invoke `tokio::fs::create_dir_all(&state.local_download_path).await`.
3. If `create_dir_all` fails, send an `AppEvent::ArchiveError` with a message explaining the failure, including the path and the error, and return early so the archive run does not continue.
4. Run standard verifications (`cargo check`, `cargo fmt`, `cargo clippy`, `cargo test`) to ensure everything is correct.
