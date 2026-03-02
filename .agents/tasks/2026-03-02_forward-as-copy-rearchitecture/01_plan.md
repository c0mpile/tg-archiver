# Plan: Forward-as-Copy Rearchitecture

1. Modify `State` schema:
   - Remove `download_status`, `local_download_path`, `filters`.
   - Add `last_forwarded_message_id`: Option<i32> and `source_message_count`: Option<i32>.
   - Keep `post_count_threshold` instead of full filters.

2. Refactor `TelegramClient`:
   - Delete `upload_media` and `get_media_description`.
   - Add `forward_messages_as_copy` using `messages.ForwardMessages` TL function with `drop_author: true`.
   - Add `create_topic` using `messages.CreateForumTopic`.

3. Rewire archive logic in `run_archive_loop`:
   - Use ID-Range Chunking (fetch chunks of 100 messages oldest-to-newest by ID starting from `last_forwarded_message_id + 1`).
   - Filter out service/empty messages.
   - Delay 500ms between chunks.

4. Update UI (`TUI`):
   - Replace complex filter configuration with only `post_count_threshold`.
   - Update `ArchiveProgress` to show message count/progress instead of active downloads.
   - Provide an option to auto-create a destination topic if one isn't selected.

5. Verification:
   - Cargo fmt, clippy, build, test.
