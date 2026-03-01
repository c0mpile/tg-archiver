# Plan for Subtask 8: Destination Topic Uploading

1. Update `telegram/mod.rs`: Add `upload_media` method.
2. Ensure `upload_media` uploads a local file leveraging `client.upload_file` and sends a new message containing the media to a specific `dest_topic_id`.
3. Wrap both `upload_file` and `send_message` with `retry_flood_wait!`. Re-architect `retry_flood_wait!` slightly to comfortably match standard API errors containing IO or structural wrappers.
4. Update `state/mod.rs` with a new `Uploaded` variant to differentiate complete files versus completely synced up files.
5. In `archive/mod.rs`, check for `Needs Upload` based on configured properties, and trigger `upload_media`.
6. Reflect the `Uploaded` state properly in `tui/archive_progress.rs` for tracking.
7. Perform surgical verifications via `cargo fmt`, `cargo clippy`, and `cargo test`.
