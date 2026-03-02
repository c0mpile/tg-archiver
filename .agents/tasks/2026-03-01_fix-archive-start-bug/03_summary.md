# Summary: Fix Archive Start Bug

## Root Cause
The archive run logic in `start_archive_run` (in `src/archive/mod.rs`) assumed that the local download directory specified by `state.local_download_path` already existed. However, if the user provided a non-existent path, `tokio::fs::File::create(&file_path)` inside the individual download workers would instantly fail with an OS error 2 ("No such file or directory"), causing all files to transition into a `Failed` state rather than initiating their download.

## Fix
A call to `tokio::fs::create_dir_all(&state.local_download_path).await` was added to `start_archive_run` before the main `run_archive_loop` starts. 
Now, at the very beginning of the archive run, the application ensures the download directory (and its parent directories) are created. If the directory creation fails (e.g., due to permission issues), the process catches the error and emits an `AppEvent::ArchiveError` with a clear message stating the failure, aborting the archive run instead of spamming failures across all worker threads. 

Validation checks `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo build --release` all passed successfully.
