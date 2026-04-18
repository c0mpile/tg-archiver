# Summary — 2026-04-18_h265-transcode

**Status:** Complete ✓  
**File changed:** `src/upload/mod.rs` (only)

---

## What was done

### Output format change: MKV → H265.MP4

`transcode_to_h265` now writes to `<stem>.h265.mp4` instead of `<stem>.mkv`.
The `.h265` stem suffix avoids collision with the source `<stem>.mp4`.
No ffmpeg arguments changed — H.265/HEVC in an MP4 container is valid.

Three sites updated in `transcode_to_h265`:
1. Path derivation (early-return existence check)
2. ffmpeg `-y <output>` argument
3. Return value

One site updated in `run_upload_loop`:
- Short-circuit "already transcoded" gate checks for `.h265.mp4` instead of `.mkv`

### Inline video attributes

`upload_file` now injects proper Telegram video metadata for MP4 files so
the file is embedded as inline video rather than a generic document attachment.

**New helper `get_video_metadata(path) -> Option<(i32, i32, f64)>`:**
- Runs `ffprobe` to extract `width`, `height`, `duration` from the first video stream
- Returns `None` on any failure; `upload_file` degrades gracefully (uploads as document)
- No new crate dependencies

**`upload_file` changes:**
- Calls `get_video_metadata` for any `.mp4` file (case-insensitive)
- Builds `attributes` vec:
  - `DocumentAttributeFilename` always present (both branches)
  - `DocumentAttributeVideo { supports_streaming: true, duration: f64, w: i32, h: i32, video_codec: None }` added when metadata is available
- Replaces previous `attributes: vec![]`

### Type verification

`DocumentAttributeVideo` in grammers-tl-types 0.9.0 (verified from build output):
- `duration: f64` — parsed and used directly, no cast
- `w: i32`, `h: i32` — not `u32`
- `video_codec: Option<String>` — extra field not in TL schema docs; set to `None`

---

## Verification

| Check | Result |
|---|---|
| `cargo fmt -- --check` | ✓ exit 0 |
| `cargo clippy -- -D warnings` | ✓ exit 0 |
| `cargo build --release` | ✓ exit 0 (5.11s) |
| `cargo test -- --test-threads=1` | ✓ 2 passed, 1 ignored |
