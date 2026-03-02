# Summary: Fix Archive Start Bug (Proper Caching)

## Root Cause
The archive start bug occurred because starting an archive session (`AppEvent::StartArchiveRun`) does not have the `source_channel_id` mapped inside the `telegram_client.peer_cache`. If the program is freshly started from a `state.json` file, the `peer_cache` is inherently empty.

Calling `telegram_client.get_input_peer()` in `start_archive_run` returned `None`, which triggered an error and ended the active archive thread immediately. While the previous fix merely surfaced this error to the user interface, it was an undesired user experience as it required them to "warm up" the peer cache themselves without clear technical reasons for doing so.

## Fix Implemented
In `src/app/mod.rs`, when the UI handles `AppEvent::StartArchiveRun` or `AppEvent::PromptResumeResult(true)`:
Instead of directly passing execution to `crate::archive::start_archive_run(...)`, the application instantly transitions the UI to the `ActiveView::ArchiveProgress` allowing an implicitly perceived "Loading..." state.

Concurrently, a `tokio::spawn` wrapper checks:
`tg_clone.get_input_peer(source_id).await.is_none()`
If true, it awaits both `tg_clone.get_joined_channels().await` and `tg_clone.get_joined_groups().await` to populate `peer_cache`.

After making sure the required peer ID resides in the memory cache, execution resumes with `crate::archive::start_archive_run(...)` without any abrupt app state transitions or errors displayed to the user.

## Verification Checks
- [x] Build is green (`cargo build --release` ok)
- [x] Linter passed (`cargo clippy -- -D warnings` ok)
- [x] Formatter passed (`cargo fmt -- --check` ok)
- [x] Tests passed (`cargo test` ok)
- [x] No unrelated code touched
- [x] Trace files saved in `.agents/tasks/`
