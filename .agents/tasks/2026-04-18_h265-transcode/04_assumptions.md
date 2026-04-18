# Assumptions & Shortcuts — 2026-04-18_h265-transcode

---

## Process Shortcuts

### 1. Task audit files created after implementation, not before

**Should have:** Created `01_plan.md`, `02_terminal.log`, `03_summary.md` in
`.agents/tasks/2026-04-18_h265-transcode/` at the start of the task.

**What happened:** Wrote the plan to the Antigravity artifacts directory
(`~/.gemini/antigravity/brain/<conversation-id>/implementation_plan.md`) instead
of the task folder. Did not create the `.agents/tasks/` files at all until the
user explicitly called it out after implementation was complete. The terminal
log entries and summary were reconstructed from memory after the fact rather
than captured in real time.

---

## Unspecified Decisions in Implementation

### 2. `DocumentAttributeFilename` added to non-MP4 uploads as well

**Spec said:** Add `DocumentAttributeFilename` alongside `DocumentAttributeVideo`
for MP4 files.

**Assumption:** Extended it to the `else` branch too (non-MP4 files get
`DocumentAttributeFilename` only, no `DocumentAttributeVideo`). The original
code had a bare `attributes: vec![]` with no filename attribute at all.

**Rationale:** `DocumentAttributeFilename` is standard for all document uploads
and its absence was an existing omission in the original code. Adding it to both
branches is correct behaviour and matches Telegram client conventions. Noted in
the plan but not explicitly requested in the task spec.

### 3. `video_codec: None` for the undocumented struct field

**Spec said:** The plan listed `DocumentAttributeVideo` fields from the TL schema.
`video_codec: Option<String>` was not in the original plan or the task spec.

**Discovery:** Found during type verification — the generated struct has an extra
`video_codec` field not documented in the public TL schema.

**Assumption:** Set to `None`. No codec string is needed for the forwarding use
case and `None` is the safe default.

### 4. `nosound: false`, `round_message: false` defaults

**Spec said:** Set `supports_streaming: true`. Did not specify values for other
bool fields.

**Assumption:** Both set to `false`. These are structurally obvious for
non-silent, non-circular videos. No alternative value makes sense for this use
case.

### 5. `preload_prefix_size: None`, `video_start_ts: None` defaults

**Spec said:** These optional fields were listed in the plan with `None`.

**Assumption:** `None` for both. `preload_prefix_size` is a Telegram-internal
streaming optimization hint that the server can compute; `video_start_ts` is only
relevant for timestamp-seeking previews. Neither is needed for basic inline
playback.

### 6. ffprobe stream selector `v:0` — first video stream only

**Spec said:** "Runs ffprobe … to extract width, height, duration."

**Assumption:** Used `-select_streams v:0` which selects only the first video
stream. For files with multiple video streams (rare in practice), only the first
stream's dimensions and duration are used. The spec did not specify which stream
to select.

### 7. `ext.to_lowercase()` called twice — not deduplicated

**Observation:** The mime-type match block calls `ext.to_lowercase().as_str()`
and then `is_mp4` recomputes `ext.to_lowercase() == "mp4"` independently. A
single `let ext_lower = ext.to_lowercase();` binding could serve both.

**Shortcut taken:** Did not refactor the existing mime-type block. The duplicate
call is on a short string and has no measurable cost; changing it would touch
lines outside the task scope, violating the surgical precision rule.

### 8. `get_video_metadata` called for all MP4 uploads, not only transcoded ones

**Spec said:** "Call `get_video_metadata(&path).await` for any file with a `.mp4`
extension."

**Assumption:** Implemented exactly as specified — probe runs on every MP4
regardless of size or whether it was transcoded. This means small MP4s well
below the 4 GiB threshold also get probed. This is correct per spec but does
add one `ffprobe` subprocess per small MP4 upload. No cap or size gate was
added because none was specified.
