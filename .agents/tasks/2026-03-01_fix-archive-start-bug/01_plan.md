# Plan: Fix Archive Start Bug Properly (Transparent Cache Warming)

## Root Cause
The previous fix correctly handled the `ArchiveError` surfacing it to the Home screen. However, on a fresh startup, the `TelegramClient` `peer_cache` is empty. If the user hits 's' (or chooses to Resume from the Prompt), `start_archive_run` fails because `get_input_peer` returns `None`. We need to silently warm the cache if necessary without throwing the user back to Home.

## Proposed Fix
In `src/app/mod.rs` when handling `AppEvent::StartArchiveRun` and `AppEvent::PromptResumeResult(true)`:
1. Instead of immediately calling `start_archive_run`, spawn an async task.
2. In the spawned task, check if `telegram.get_input_peer(source_channel_id).await` is `Some`.
3. If it's `None`, call `telegram.get_joined_channels().await` and `telegram.get_joined_groups().await`. This will fetch the latest channels and groups and populate the `peer_cache`.
4. After fetching (or if it was already cached), call `crate::archive::start_archive_run(...)`.
5. This ensures the TUI transitions to `ArchiveProgress` immediately, and the archive starts successfully even on a fresh launch. Note that during the fetch delay, the `ArchiveProgress` view will simply show 0 active downloads, which acts as an implicit "Preparing..." state. We can optionally send an `AppEvent::DownloadProgress` with a fake message or just let the progress screen sit empty for the ~1 second it takes to fetch.

Actually, doing it inside `ArchiveProgress` is exactly what the user asked: "The user should see a "Preparing..." or "Loading channel data..." message in the progress view while this happens rather than being dropped back to Home."

To show "Preparing..." we can tweak `ArchiveProgress` to show it if something is loading, or just let the worker pool handle the delay. A simpler way is to dispatch a special `LoadingCache` status, but the easiest surgical change is to just delay `start_archive_run` inside a spawned task in `handle_event`.

### Changes to `src/app/mod.rs`:
```rust
            AppEvent::StartArchiveRun => {
                self.active_view = ActiveView::ArchiveProgress;
                
                let state_clone = self.state.clone();
                let tg_clone = Arc::clone(telegram);
                let tx_clone = tx.clone();
                let paused_clone = Arc::clone(&self.is_paused);
                
                if let Some(source_id) = self.state.source_channel_id {
                    tokio::spawn(async move {
                        // Warm up cache if needed
                        if tg_clone.get_input_peer(source_id).await.is_none() {
                            let _ = tg_clone.get_joined_channels().await;
                            let _ = tg_clone.get_joined_groups().await;
                        }
                        
                        crate::archive::start_archive_run(
                            state_clone,
                            tg_clone,
                            tx_clone,
                            paused_clone,
                        );
                    });
                }
            }
```
Apply the same to `AppEvent::PromptResumeResult(true)`.

## Verification
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` pass.
