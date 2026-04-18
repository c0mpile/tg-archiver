# H.265 VAAPI Transcode — Summary

## What was done

Added automatic H.265 VAAPI transcoding for oversized MP4 uploads. Three files modified, no files created or deleted.

### `src/upload/mod.rs`

- **`get_file_duration`** (private): runs `ffprobe` via `tokio::process::Command` to retrieve video duration in seconds; caller uses `.unwrap_or(0.0)` for graceful degradation.
- **`transcode_to_h265`** (pub): spawns `ffmpeg` with the VAAPI H.265 command; short-circuits if the `.mkv` already exists; streams stderr line-by-line, parsing `fps=`, `time=`, `speed=` tokens to emit `AppEvent::TranscodeProgress`; sends `TranscodeComplete` on success, returns `Err` on non-zero exit.
- **`run_upload_loop`**: before `upload_file`, checks `size_bytes > 4 GiB && ext == "mp4"`. If true: checks for existing `.mkv` (skips transcode if present), otherwise sends `TranscodeStarted`, probes duration, calls `transcode_to_h265`. On `Err`: sends `TranscodeError` (to clear App UI state) then `UploadWarning` (consistent with all other warning paths) and `continue`s. `upload_path` is the `.mkv`; `rel_path_for_state` is always the original `.mp4` relative path for correct sync tracking.

### `src/app/mod.rs`

- Four new `AppEvent` variants: `TranscodeStarted`, `TranscodeProgress`, `TranscodeComplete`, `TranscodeError`.
- Six new `App` fields: `upload_is_transcoding`, `upload_transcode_filename`, `upload_transcode_fps`, `upload_transcode_speed`, `upload_transcode_time_encoded`, `upload_transcode_percent`. Initialised to defaults in `App::new`.
- Four new match arms in `handle_event`. `TranscodeError` only resets UI state; the worker sends `UploadWarning` directly (no re-dispatch indirection).

### `src/tui/upload.rs`

`render_upload_progress` builds its constraint vector dynamically. When `upload_is_transcoding` is true, a `Constraint::Length(6)` block is prepended and offset indexing (`offset = usize::from(app.upload_is_transcoding)`) keeps all subsequent chunk references correct. The transcode panel shows a yellow-bordered block with filename/FPS/speed/time stats and a `Color::Yellow` Gauge.

## Verification

```
cargo fmt -- --check    ✓
cargo clippy -- -D warnings   ✓
cargo build --release   ✓  (5.84s)
cargo test -- --test-threads=1  ✓  (2 passed, 1 ignored)
```
