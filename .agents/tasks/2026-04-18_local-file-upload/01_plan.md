# 01_plan.md: Local File Upload

## New Files
- `src/upload/mod.rs`: Contains `upload_file` worker, `UploadSyncState` and `UploadedFile` structs for state persistence, and the FNV-1a hash implementation.
- `src/tui/upload.rs`: Contains all TUI rendering logic for the new `ActiveView::Upload...` variants.

## Files Modified
- `src/app/mod.rs`: Add new `ActiveView` variants, `AppEvent` variants, new fields to the `App` struct, and event handling logic for the `u` key and upload workflows.
- `src/tui/mod.rs`: Export the new `upload` module and dispatch rendering for the new views in the main `render` match.
- `src/main.rs`: Declare `mod upload;`.

## Upload Worker Lifecycle
- The user initiates upload from the TUI. The TUI sends an `AppEvent::StartUploadRun`.
- The main event loop receives `StartUploadRun`, updates the view to `ActiveView::UploadProgress`, and spawns a background tokio task (the upload worker).
- The worker iterates over the flat list of files to upload. For each file:
  - It checks if the mode is `Sync` and the file is already uploaded with a size >= current size. If so, it skips it.
  - It calls `upload_file` (raw TL `messages.SendMedia`) with a 500ms delay between uploads, wrapped in `retry_flood_wait!`.
  - It sends `AppEvent::UploadFileComplete` back to the main loop.
- The main loop handles `UploadFileComplete` by updating the progress bar. If in `Sync` mode, it appends the file to `upload_sync_state` and saves it atomically to disk.
- Once all files are processed, the worker sends `AppEvent::UploadComplete`.
- The main loop handles `UploadComplete` and shows completion status on the progress view.
- In case of a fatal error, the worker sends `AppEvent::UploadError(msg)`, which the main loop displays.

## Subdirectory Expansion
- On `u` from Home, `tokio::fs::read_dir` is used non-recursively on the CWD to populate `upload_entries`. Directories are marked as `UploadEntry::Dir`.
- When the user confirms the selection and starts the upload run, a recursive resolution step happens:
  - We iterate over the selected `UploadEntry` items.
  - For `File`s, we add them to a flat list.
  - For `Dir`s, we spawn a helper that recursively uses `tokio::fs::read_dir` to find all files within that directory.
  - The resulting flat list is sorted alphabetically within each directory structure.

## FNV-1a Hash
Since the `sha2` crate is not in `Cargo.toml`, we will implement a simple inline FNV-1a hash function in `src/upload/mod.rs` to generate the 8-hex-character CWD disambiguator for the state filename.

## Open Questions
- Should we provide a way to clear the `UploadSyncState` for a directory if the user wants to start completely fresh? (Currently, 'n' ignores it but doesn't delete it).
- What should happen if a file is unreadable during the recursive expansion or upload phase? (Skip with a warning or abort the entire run?)
