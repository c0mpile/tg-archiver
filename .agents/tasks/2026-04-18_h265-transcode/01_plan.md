# Task: Change Transcoded Output from MKV to H265.MP4

**Date:** 2026-04-18  
**Scope:** `src/upload/mod.rs` only — no other files touched.

---

## Problem

`transcode_to_h265` writes its output to `<stem>.mkv`. Two issues:

1. Telegram does not inline-play MKV — only MP4.
2. Naively changing to `.mp4` would conflict with the source file.

---

## Pre-Code Type Verification

Checked `DocumentAttributeVideo` from the compiled build output at:
`target/debug/build/grammers-tl-types-09f2483fb741d69a/out/generated_types.rs`

```rust
pub struct DocumentAttributeVideo {
    pub round_message: bool,
    pub supports_streaming: bool,
    pub nosound: bool,
    pub duration: f64,              // ← f64, NOT u32
    pub w: i32,                     // ← i32, NOT u32
    pub h: i32,                     // ← i32, NOT u32
    pub preload_prefix_size: Option<i32>,
    pub video_start_ts: Option<f64>,
    pub video_codec: Option<String>, // ← extra field not in original plan
}
```

**Mandatory adjustments from original plan:**
- `get_video_metadata` returns `(i32, i32, f64)` — not `(u32, u32, u32)`
- `duration` is parsed as `f64` and used directly — no `.round() as u32`
- `w` and `h` parse as `i32` directly
- `video_codec: None` included in struct literal

---

## Changes

### 1. Imports

Add `DocumentAttribute`, `DocumentAttributeFilename`, `DocumentAttributeVideo`
to the existing `grammers_tl_types` import lines.

### 2. `upload_file` — video attribute injection

After the mime-type match block, for MP4 files:
- Call `get_video_metadata(local_path).await`
- Build `attributes` vec:
  - Always includes `DocumentAttributeFilename`
  - If metadata available: also includes `DocumentAttributeVideo { supports_streaming: true, duration: f64, w: i32, h: i32, video_codec: None, … }`
- Replace `attributes: vec![]` in `InputMediaUploadedDocument` with the built vec

### 3. New helper — `get_video_metadata`

```rust
async fn get_video_metadata(path: &Path) -> Option<(i32, i32, f64)>
```

- Runs `ffprobe -v error -select_streams v:0 -show_entries stream=width,height,duration -of default=noprint_wrappers=1:nokey=0`
- Parses `key=value` lines for `width` (i32), `height` (i32), `duration` (f64)
- Returns `None` on any failure — caller degrades gracefully (upload continues as generic document)
- No new crate dependencies — uses existing `tokio::process::Command`

### 4. `transcode_to_h265` — output path

Change from `input_path.with_extension("mkv")` to:
```rust
let stem = input_path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
let h265_path = input_path.with_file_name(format!("{}.h265.mp4", stem));
```
Update: early-return existence check, ffmpeg `-y <output>` arg, return value.

### 5. `run_upload_loop` — short-circuit gate

Replace `.with_extension("mkv")` existence check with the same
`.with_file_name(format!("{}.h265.mp4", stem))` pattern.

---

## Verification Plan

```
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --release
cargo test -- --test-threads=1
```

All must exit 0.
