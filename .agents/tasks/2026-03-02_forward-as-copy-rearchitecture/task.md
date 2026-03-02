# Forward-as-Copy Rearchitecture

- [ ] 1. Update State Schema
  - [ ] Remove `Filters` (replaced with `post_count_threshold: u32`)
  - [ ] Remove `download_status`, `local_download_path`, `message_cursor`
  - [ ] Add `last_forwarded_message_id: Option<i32>`
  - [ ] Add `source_message_count: Option<i32>`
  - [ ] Add `post_count_threshold: u32`

- [ ] 2. Telegram Client Modifications
  - [ ] Add `create_topic` method using raw TL `messages::CreateForumTopic`
  - [ ] Add `forward_messages_as_copy` method using raw TL `messages::ForwardMessages` with `drop_author: true`

- [ ] 3. TUI & App State Updates
  - [ ] Remove complex filter configuration widgets
  - [ ] Add `post_count_threshold` configuration widget
  - [ ] Modify `ArchiveProgress` to show message count instead of bytes downloaded
  - [ ] Add "Create new topic automatically" to Destination topic picker
  - [ ] Handle new topic creation in `App` state machine

- [ ] 4. Archive Worker Rearchitecture
  - [ ] Remove download worker pool entirely
  - [ ] Determine newest `message_id` at start of run
  - [ ] If `post_count_threshold > 0` and no resuming, find the starting message ID
  - [ ] Iterate in chunks of 100 IDs (from `start_id` to `newest_id`)
  - [ ] Request chunks using `get_messages_by_id`
  - [ ] Filter out service messages
  - [ ] Forward chunks using `forward_messages_as_copy`
  - [ ] Wait 500ms between batches
  - [ ] Persist `last_forwarded_message_id` after each chunk
