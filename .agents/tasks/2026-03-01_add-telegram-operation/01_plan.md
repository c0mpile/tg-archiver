# Plan for Add Telegram Operation (Description Heuristic)
1. Add `get_media_description` to the `TelegramClient` struct in `src/telegram/mod.rs` applying flood-wait retry helper to the underlying API call `get_messages_by_id`.
2. Add a `caption` string option field to the `DownloadStatus::Complete` variant in `src/state/mod.rs` to persist descriptions.
3. Call `get_media_description` in the parallel worker pool in `src/archive/mod.rs` if `filters.include_text_descriptions` is enabled.
4. Save the resultant description string as a `.txt` alongside the respective media file using `tokio::fs::write`.
5. Prepend the gathered string into the upload caption inside `DownloadStatus::Complete` to make it accessible to Subtask 8.
6. Assure `cargo test`, `cargo clippy`, and `cargo fmt` pass without regression.
