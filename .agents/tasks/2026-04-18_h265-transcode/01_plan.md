# H.265 VAAPI Auto-Transcode for Oversized Upload Files

## Background

The upload worker (`run_upload_loop`) currently uploads files as-is. Any file
≥ 4 GiB that is an `.mp4` must first be transcoded to H.265 MKV via ffmpeg
(VAAPI) before upload. The MKV replaces the source in the upload call; the
original `.mp4` is never deleted, and sync-state tracks the original filename.

---

## Proposed Changes

### `src/upload/mod.rs`

#### [MODIFY] [mod.rs](file:///home/kevin/dev/tg-archiver/repo/src/upload/mod.rs)

**New helper — `get_file_duration`** (private async fn)

- Runs `ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 <path>`
  via `tokio::process::Command` with `stdout: Stdio::piped()`, `stderr: Stdio::null()`
- Awaits the child, captures stdout, trims, parses as `f64`
- Returns `Err` on spawn failure, non-zero exit, or parse failure
- Caller always uses `.unwrap_or(0.0)` — no crash if `ffprobe` is absent

**New public fn — `transcode_to_h265`**

```rust
pub async fn transcode_to_h265(
    input_path: &Path,
    app_tx: &tokio::sync::mpsc::Sender<AppEvent>,
    filename: &str,
    index: usize,
    total: usize,
    total_duration_secs: f64,
) -> anyhow::Result<std::path::PathBuf>
```

- Derives `mkv_path = input_path.with_extension("mkv")`
- If `mkv_path.exists()` -> returns `Ok(mkv_path)` immediately (no ffmpeg)
- Spawns ffmpeg via `tokio::process::Command`:
  `ffmpeg -nostdin -vaapi_device /dev/dri/renderD128 -i <input> -vf 'format=nv12,hwupload' -vcodec hevc_vaapi -rc_mode QVBR -global_quality 22 -b:v 3000k -maxrate 6000k -refs 4 -g 120 -bf 3 -acodec copy -y <output.mkv>`
  with `stdin: Stdio::null()`, `stdout: Stdio::null()`, `stderr: Stdio::piped()`
- Reads stderr via `tokio::io::BufReader` + `AsyncBufReadExt::lines()` in a
  concurrent task (`tokio::spawn`); sends `AppEvent::TranscodeProgress` per
  parsed progress line
- **Progress parsing** targets ffmpeg's stats line format; regex-free: split
  on whitespace tokens, scan for `fps=`, `time=`, `speed=` key=value pairs
- `percent = time_encoded_secs / total_duration_secs * 100.0`, clamped 0-100;
  stays 0 if `total_duration_secs == 0.0`
- After the progress reader task completes, awaits child exit status
- Non-zero exit -> `Err(anyhow!("ffmpeg exited with status {}", code))`
- Success -> `Ok(mkv_path)`

**Modified — `run_upload_loop`**

Before the existing `upload_file` call, add a transcode gate:

```
if size_bytes > 4 GiB && extension == "mp4" (case-insensitive):
    let mkv = path.with_extension("mkv")
    if !mkv.exists():
        send TranscodeStarted
        let duration = get_file_duration(&path).await.unwrap_or(0.0)
        match transcode_to_h265(...).await:
            Ok(mkv_path) -> upload_path = mkv_path; send TranscodeComplete
            Err(e) -> send TranscodeError; continue
    else:
        upload_path = mkv
    rel_path_for_state = rel_path (original .mp4 rel path)
else:
    upload_path = path
    rel_path_for_state = rel_path
```

- `upload_file` is called with `upload_path` (possibly `.mkv`)
- All sync-state bookkeeping uses `rel_path_for_state` (original `.mp4` relative
  path), so incremental sync correctly identifies already-processed files
- Caption derived from `path.file_stem()` (unchanged, stems are identical)
- `TranscodeError` skips the file (non-fatal); already routed via `UploadWarning`

---

### `src/app/mod.rs`

#### [MODIFY] [mod.rs](file:///home/kevin/dev/tg-archiver/repo/src/app/mod.rs)

**New `AppEvent` variants** (appended to the existing enum):

```rust
TranscodeStarted { filename: String, index: usize, total: usize },
TranscodeProgress {
    filename: String,
    fps: f32,
    speed: f32,
    time_encoded: String,
    percent: f32,
},
TranscodeComplete { filename: String, mkv_path: std::path::PathBuf },
TranscodeError { filename: String, error: String },
```

**New `App` fields** (after `upload_cancel_tx`):

```rust
pub upload_is_transcoding: bool,
pub upload_transcode_filename: String,
pub upload_transcode_fps: f32,
pub upload_transcode_speed: f32,
pub upload_transcode_time_encoded: String,
pub upload_transcode_percent: f32,
```

All initialised to defaults in `App::new`.

**New event handlers** in `App::handle_event` (after `UploadTopicCreated`):

- `TranscodeStarted`: set `upload_is_transcoding = true`, set filename, reset other fields to 0
- `TranscodeProgress`: update all transcode fields
- `TranscodeComplete`: set `upload_is_transcoding = false`, clear all transcode fields
- `TranscodeError`: set `upload_is_transcoding = false`; `tx.try_send(AppEvent::UploadWarning(...))`

---

### `src/tui/upload.rs`

#### [MODIFY] [upload.rs](file:///home/kevin/dev/tg-archiver/repo/src/tui/upload.rs)

Modify `render_upload_progress` to conditionally prepend a transcode block.

**Layout strategy:** build constraint list dynamically:

```rust
let mut constraints = vec![];
if app.upload_is_transcoding {
    constraints.push(Constraint::Length(6)); // transcode panel
}
constraints.push(Constraint::Length(3)); // file info
constraints.push(Constraint::Length(3)); // upload gauge
constraints.push(Constraint::Min(1));    // warnings/help
```

When transcoding: render a `Block` titled `"Transcoding"`, containing a
filename line, `FPS: {fps:.1}  Speed: {speed:.1}x  Encoded: {time}`, and a
`Gauge` with `Color::Yellow` fill at `upload_transcode_percent`.

---

## Transcode/Upload Lifecycle (Oversized MP4)

```
run_upload_loop iterates files
  file is .mp4 AND size > 4 GiB
    mkv_path exists? -> use it directly (skip transcode)
    mkv_path absent:
      send TranscodeStarted  -> App: upload_is_transcoding = true
      get_file_duration (ffprobe) -> total_duration_secs
      transcode_to_h265 spawns ffmpeg
        stderr reader loop -> send TranscodeProgress per parsed line
      ffmpeg exits 0?
        send TranscodeComplete -> App: upload_is_transcoding = false
        upload_file(mkv_path, rel_path_for_state=original_mp4_rel)
      ffmpeg exits non-0?
        send TranscodeError
        App re-emits as UploadWarning; file skipped
```

---

## ffmpeg stderr Parsing Strategy

ffmpeg writes progress to stderr:
```
frame=  123 fps= 45 q=28.0 size=   12345kB time=00:00:10.50 bitrate= 100kbits/s speed=2.5x
```

Regex-free: split line on whitespace, iterate tokens, match prefix `fps=`, `time=`, `speed=`:
- `fps=<val>` -> parse f32
- `time=HH:MM:SS.ss` -> h*3600 + m*60 + s = time_secs; keep string for display
- `speed=<val>x` -> strip trailing 'x', parse f32

`percent = (time_secs / total_duration_secs * 100.0).clamp(0.0, 100.0)`; 0 if duration is 0.

---

## Open Questions

> [!IMPORTANT]
> **`TranscodeError` double-dispatch** — The spec says `App::handle_event` for
> `TranscodeError` should `tx.try_send(AppEvent::UploadWarning(...))`. However,
> the worker itself also has access to `app_tx` and could send `UploadWarning`
> directly from `run_upload_loop` after receiving the `Err` from
> `transcode_to_h265`. The spec's design routes the warning through App, which
> means `TranscodeError` is fired from the worker and App re-emits it. The
> plan follows the spec exactly. **No change needed — flagged for awareness.**

> [!NOTE]
> **VAAPI device path** — Hardcoded to `/dev/dri/renderD128` per spec.
> Not configurable. Future config-field work would be a separate task.

> [!NOTE]
> **Sync-state size tracking** — Sync state records `size_bytes` of the
> original `.mp4`. On next sync, `.mp4` still exists at the same size, so
> the file is correctly skipped. MKV existence short-circuits ffmpeg.

---

## Verification Plan

### Automated

```
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --release
cargo test -- --test-threads=1
```

All must exit 0.

### Manual (post-build sanity)

- Confirm `upload_is_transcoding` initialised in `App::new`
- Confirm all four new `AppEvent` variants covered in `handle_event`
- Confirm dynamic layout in `render_upload_progress` compiles without
  chunk index misalignment
