# Assumptions & Shortcuts - Fix Upload Bugs

Summarizing decisions, assumptions, and shortcuts made during the fix of upload bugs in `src/app/mod.rs`.

## Decisions & Assumptions

1.  **Atomic Ordering Consistency:** The user requested `Ordering::SeqCst` for the `p` key handler in `ActiveView::UploadProgress`. I assumed this higher synchronization level should be used consistently throughout the new upload state management (e.g., resetting `is_paused` to `false` on cancel), even where existing archive code might use `Relaxed`.
2.  **Error Propagation for Topic Creation:** In the `ActiveView::UploadTopicNameEntry` handler, if `create_topic` fails, I maintained the existing assumption of sending `AppEvent::TopicsLoaded(Err(e.to_string()))`. While a specialized `UploadTopicError` might be cleaner, I stuck to the existing pattern to minimize diff noise.
3.  **UI State on Completion:** I assumed that on `AppEvent::UploadComplete`, the progress bar should be "filled" (`current = total`) even if some files were skipped or the counts weren't perfectly aligned, to provide clear visual closure to the user.
4.  **Implicit Sender Drop:** I assumed that setting `self.upload_pause_tx` and `self.upload_cancel_tx` to `None` is the preferred way to signal the background task's receivers that the control halves are gone, relying on standard Rust drop semantics rather than explicit signaling.

## Shortcuts

1.  **Surgical Formatting:** `cargo fmt -- --check` failed on the entire repository due to pre-existing formatting issues in `src/tui/upload.rs` and `src/upload/mod.rs`. To comply with the "Surgical Modification" rule and stay within scope, I took the shortcut of only formatting `src/app/mod.rs` specifically to satisfy the verification requirement for my own changes.
2.  **Existing Monitoring Logic in `UploadProgress`:** The original `ActiveView::UploadProgress` handler had a logic error where it tried to use `monitoring_cancel_tx` (likely a copy-paste from archive/monitoring views). I surgically replaced this with the correct `upload_cancel_tx` logic without refactoring the surrounding view structure.
