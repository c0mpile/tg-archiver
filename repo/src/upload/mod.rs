use crate::app::{AppEvent, UploadEntry, UploadMode};
use crate::telegram::TelegramClient;
use anyhow::{Context, Result, anyhow};
use grammers_tl_types::enums::{DocumentAttribute, InputMedia, InputReplyTo};
use grammers_tl_types::functions::messages::SendMedia;
use grammers_tl_types::types::{
    DocumentAttributeFilename, DocumentAttributeVideo, InputMediaUploadedDocument,
    InputReplyToMessage,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

#[derive(Serialize, Deserialize, Clone)]
pub struct UploadSyncState {
    pub cwd: String,
    pub dest_group_id: i64,
    pub dest_group_title: String,
    pub dest_topic_id: Option<i32>,
    pub dest_topic_title: Option<String>,
    #[serde(default)]
    pub uploaded_files: Vec<UploadedFile>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UploadedFile {
    pub filename: String,
    pub size_bytes: u64,
}

impl UploadSyncState {
    pub async fn load(cwd: &Path) -> Result<Option<Self>> {
        let hash = fnv1a_hash(cwd.to_string_lossy().as_ref());
        let file_name = format!("sync-{:08x}.json", hash);
        let mut path = get_state_dir().await?;
        path.push(file_name);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .context("Failed to read sync state file")?;
        let state: UploadSyncState =
            serde_json::from_str(&content).context("Failed to deserialize sync state")?;
        Ok(Some(state))
    }

    pub async fn save(&self, cwd: &Path) -> Result<()> {
        let hash = fnv1a_hash(cwd.to_string_lossy().as_ref());
        let file_name = format!("sync-{:08x}.json", hash);
        let mut path = get_state_dir().await?;
        path.push(file_name);

        let tmp_path = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&tmp_path, content)
            .await
            .context("Failed to write temp sync state")?;
        fs::rename(&tmp_path, &path)
            .await
            .context("Failed to rename temp sync state")?;
        Ok(())
    }
}

pub async fn get_state_dir() -> Result<PathBuf> {
    let mut state_dir = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut home = PathBuf::from(std::env::var("HOME").expect("HOME env var not set"));
            home.push(".local");
            home.push("state");
            home
        });
    state_dir.push("tg-archiver");
    if !state_dir.exists() {
        fs::create_dir_all(&state_dir)
            .await
            .context("Failed to create state directory")?;
    }
    Ok(state_dir)
}

fn fnv1a_hash(s: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash & 0xFFFFFFFF
}

pub async fn upload_file(
    client: &TelegramClient,
    local_path: &Path,
    dest_group_id: i64,
    dest_topic_id: Option<i32>,
    caption: &str,
) -> Result<()> {
    let file_name = local_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut file = tokio::fs::File::open(local_path)
        .await
        .context("Failed to open file for upload")?;
    let file_size = file
        .metadata()
        .await
        .context("Failed to read file metadata")?
        .len() as usize;
    let uploaded = client
        .client
        .upload_stream(&mut file, file_size, file_name)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to upload file to Telegram server: {:#}", e))?;

    let ext = local_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mime_type = match ext.to_lowercase().as_str() {
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "7z" => "application/x-7z-compressed",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string();

    let is_mp4 = ext.to_lowercase() == "mp4";
    let video_meta = if is_mp4 {
        get_video_metadata(local_path).await
    } else {
        None
    };

    let filename_attr = DocumentAttribute::Filename(DocumentAttributeFilename {
        file_name: local_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    });

    let attributes = if let Some((w, h, dur)) = video_meta {
        vec![
            filename_attr,
            DocumentAttribute::Video(DocumentAttributeVideo {
                round_message: false,
                supports_streaming: true,
                nosound: false,
                duration: dur,
                w,
                h,
                preload_prefix_size: None,
                video_start_ts: None,
                video_codec: None,
            }),
        ]
    } else {
        vec![filename_attr]
    };

    let input_peer = client
        .get_input_peer(dest_group_id)
        .await
        .context("Group ID not found in memory cache.")?;

    let media = InputMedia::UploadedDocument(InputMediaUploadedDocument {
        file: uploaded.raw,
        thumb: None,
        mime_type,
        attributes,
        stickers: None,
        ttl_seconds: None,
        nosound_video: false,
        force_file: false,
        spoiler: false,
        video_cover: None,
        video_timestamp: None,
    });

    let base_micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    let reply_to = dest_topic_id.map(|id| {
        InputReplyTo::Message(InputReplyToMessage {
            reply_to_msg_id: id,
            top_msg_id: None,
            reply_to_peer_id: None,
            quote_text: None,
            quote_entities: None,
            quote_offset: None,
            monoforum_peer_id: None,
            todo_item_id: None,
        })
    });

    let req = SendMedia {
        silent: false,
        background: false,
        clear_draft: false,
        noforwards: false,
        update_stickersets_order: false,
        invert_media: false,
        allow_paid_floodskip: false,
        peer: input_peer,
        reply_to,
        media,
        message: caption.to_string(),
        random_id: base_micros,
        reply_markup: None,
        entities: None,
        schedule_date: None,
        send_as: None,
        quick_reply_shortcut: None,
        effect: None,
        allow_paid_stars: None,
        schedule_repeat_period: None,
        suggested_post: None,
    };

    crate::retry_flood_wait!(client.client.invoke(&req))
        .context("Failed to send uploaded media via raw TL")?;

    Ok(())
}

// Upload Worker
#[allow(clippy::too_many_arguments, clippy::collapsible_if)]
pub async fn run_upload_loop(
    client: std::sync::Arc<TelegramClient>,
    cwd: PathBuf,
    entries: Vec<UploadEntry>,
    selected: Vec<bool>,
    mode: UploadMode,
    dest_group_id: i64,
    dest_topic_id: Option<i32>,
    app_tx: tokio::sync::mpsc::Sender<AppEvent>,
    is_paused_rx: tokio::sync::watch::Receiver<bool>,
    cancel_rx: tokio::sync::watch::Receiver<()>,
) -> Result<()> {
    // Collect all files recursively
    let mut files_to_upload: Vec<PathBuf> = Vec::new();
    for (i, entry) in entries.into_iter().enumerate() {
        if !selected[i] {
            continue;
        }
        match entry {
            UploadEntry::File { path, .. } => files_to_upload.push(path),
            UploadEntry::Dir { path, .. } => {
                collect_files_recursive(&path, &mut files_to_upload).await?;
            }
        }
    }
    files_to_upload.sort();

    let mut state = if matches!(mode, UploadMode::Sync) {
        if let Ok(Some(s)) = UploadSyncState::load(&cwd).await {
            s
        } else {
            UploadSyncState {
                cwd: cwd.to_string_lossy().into_owned(),
                dest_group_id,
                dest_group_title: "".to_string(), // we don't strictly need these here if starting fresh without load, but app passes them normally
                dest_topic_id,
                dest_topic_title: None,
                uploaded_files: vec![],
            }
        }
    } else {
        UploadSyncState {
            cwd: cwd.to_string_lossy().into_owned(),
            dest_group_id,
            dest_group_title: "".to_string(),
            dest_topic_id,
            dest_topic_title: None,
            uploaded_files: vec![],
        }
    };

    let total = files_to_upload.len();
    for (idx, path) in files_to_upload.into_iter().enumerate() {
        // Handle cancel
        if cancel_rx.has_changed()? {
            break;
        }

        // Handle pause
        while *is_paused_rx.borrow() {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            if cancel_rx.has_changed()? {
                return Ok(());
            }
        }

        let rel_path = path
            .strip_prefix(&cwd)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        let metadata = fs::metadata(&path).await;

        let size_bytes = match metadata {
            Ok(m) => m.len(),
            Err(e) => {
                app_tx
                    .send(AppEvent::UploadWarning(format!(
                        "Skipped '{}': {}",
                        rel_path, e
                    )))
                    .await?;
                continue;
            }
        };

        if matches!(mode, UploadMode::Sync) {
            if let Some(existing) = state.uploaded_files.iter().find(|f| f.filename == rel_path) {
                if size_bytes <= existing.size_bytes {
                    // Skip
                    app_tx
                        .send(AppEvent::UploadFileComplete {
                            filename: rel_path.clone(),
                            index: idx + 1,
                            total,
                        })
                        .await?;
                    continue;
                }
            }
        }

        let caption = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown".to_string());

        // Determine actual file to upload (may be transcoded MKV)
        const FOUR_GIB: u64 = 4 * 1024 * 1024 * 1024;
        let is_oversized_mp4 = size_bytes > FOUR_GIB
            && path.extension().map(|e| e.to_ascii_lowercase())
                == Some(std::ffi::OsString::from("mp4"));

        let upload_path = if is_oversized_mp4 {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let h265_path = path.with_file_name(format!("{}.h265.mp4", stem));
            if h265_path.exists() {
                h265_path
            } else {
                app_tx
                    .send(AppEvent::TranscodeStarted {
                        filename: rel_path.clone(),
                        index: idx + 1,
                        total,
                    })
                    .await?;

                let duration = get_file_duration(&path).await.unwrap_or(0.0);

                match transcode_to_h265(&path, &app_tx, &rel_path, idx + 1, total, duration).await {
                    Ok(h265_path) => {
                        app_tx
                            .send(AppEvent::TranscodeComplete {
                                filename: rel_path.clone(),
                                mkv_path: h265_path.clone(),
                            })
                            .await?;
                        h265_path
                    }
                    Err(e) => {
                        app_tx
                            .send(AppEvent::TranscodeError {
                                filename: rel_path.clone(),
                                error: e.to_string(),
                            })
                            .await?;
                        app_tx
                            .send(AppEvent::UploadWarning(format!(
                                "Transcode failed for '{}': {}",
                                rel_path, e
                            )))
                            .await?;
                        continue;
                    }
                }
            }
        } else {
            path.clone()
        };

        if let Err(e) = upload_file(
            &client,
            &upload_path,
            dest_group_id,
            dest_topic_id,
            &caption,
        )
        .await
        {
            app_tx
                .send(AppEvent::UploadWarning(format!(
                    "Failed to upload '{}': {}",
                    rel_path, e
                )))
                .await?;
            continue;
        }

        if matches!(mode, UploadMode::Sync) {
            if let Some(existing) = state
                .uploaded_files
                .iter_mut()
                .find(|f| f.filename == rel_path)
            {
                existing.size_bytes = size_bytes;
            } else {
                state.uploaded_files.push(UploadedFile {
                    filename: rel_path.clone(),
                    size_bytes,
                });
            }
            if let Err(e) = state.save(&cwd).await {
                app_tx
                    .send(AppEvent::UploadWarning(format!(
                        "Failed to save state: {}",
                        e
                    )))
                    .await?;
            }
        }

        app_tx
            .send(AppEvent::UploadFileComplete {
                filename: rel_path.clone(),
                index: idx + 1,
                total,
            })
            .await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    app_tx.send(AppEvent::UploadComplete).await?;

    Ok(())
}

async fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut dirs_to_visit = vec![dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_visit.pop() {
        let mut entries = match fs::read_dir(&current_dir).await {
            Ok(e) => e,
            Err(_) => continue, // skip unreadable dirs silently
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                dirs_to_visit.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}

async fn get_file_duration(path: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .context("Failed to spawn ffprobe")?;

    if !output.status.success() {
        return Err(anyhow!("ffprobe exited with status {}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    trimmed
        .parse::<f64>()
        .map_err(|e| anyhow!("Failed to parse ffprobe duration '{}': {}", trimmed, e))
}

async fn get_video_metadata(path: &Path) -> Option<(i32, i32, f64)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,duration",
            "-of",
            "default=noprint_wrappers=1:nokey=0",
        ])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut duration: Option<f64> = None;

    for line in stdout.lines() {
        if let Some((key, val)) = line.split_once('=') {
            match key.trim() {
                "width" => width = val.trim().parse().ok(),
                "height" => height = val.trim().parse().ok(),
                "duration" => duration = val.trim().parse().ok(),
                _ => {}
            }
        }
    }

    Some((width?, height?, duration?))
}

pub async fn transcode_to_h265(
    input_path: &Path,
    app_tx: &tokio::sync::mpsc::Sender<AppEvent>,
    filename: &str,
    _index: usize,
    _total: usize,
    total_duration_secs: f64,
) -> Result<PathBuf> {
    let stem = input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let h265_path = input_path.with_file_name(format!("{}.h265.mp4", stem));

    if h265_path.exists() {
        return Ok(h265_path);
    }

    let mut child = Command::new("ffmpeg")
        .args(["-nostdin", "-vaapi_device", "/dev/dri/renderD128", "-i"])
        .arg(input_path)
        .args([
            "-vf",
            "format=nv12,hwupload",
            "-vcodec",
            "hevc_vaapi",
            "-rc_mode",
            "QVBR",
            "-global_quality",
            "22",
            "-b:v",
            "3000k",
            "-maxrate",
            "6000k",
            "-refs",
            "4",
            "-g",
            "120",
            "-bf",
            "3",
            "-acodec",
            "copy",
            "-y",
        ])
        .arg(&h265_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn ffmpeg")?;

    let stderr = child.stderr.take().expect("stderr was piped");
    let reader = tokio::io::BufReader::new(stderr);
    let mut lines = reader.lines();

    let tx = app_tx.clone();
    let filename_owned = filename.to_string();
    let progress_task = tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            // Parse ffmpeg progress lines of the form:
            // frame=  123 fps= 45 q=28.0 size=   12345kB time=00:00:10.50 ... speed=2.5x
            let mut fps: f32 = 0.0;
            let mut time_str = String::new();
            let mut time_secs: f64 = 0.0;
            let mut speed: f32 = 0.0;
            let mut found_time = false;

            for token in line.split_whitespace() {
                if let Some(val) = token.strip_prefix("fps=") {
                    fps = val.parse().unwrap_or(0.0);
                } else if let Some(val) = token.strip_prefix("time=") {
                    // HH:MM:SS.ss
                    time_str = val.to_string();
                    let parts: Vec<&str> = val.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let h: f64 = parts[0].parse().unwrap_or(0.0);
                        let m: f64 = parts[1].parse().unwrap_or(0.0);
                        let s: f64 = parts[2].parse().unwrap_or(0.0);
                        time_secs = h * 3600.0 + m * 60.0 + s;
                        found_time = true;
                    }
                } else if let Some(val) = token.strip_prefix("speed=") {
                    speed = val.trim_end_matches('x').parse().unwrap_or(0.0);
                }
            }

            if !found_time {
                continue;
            }

            let percent = if total_duration_secs > 0.0 {
                (time_secs / total_duration_secs * 100.0).clamp(0.0, 100.0) as f32
            } else {
                0.0
            };

            let _ = tx
                .send(AppEvent::TranscodeProgress {
                    filename: filename_owned.clone(),
                    fps,
                    speed,
                    time_encoded: time_str,
                    percent,
                })
                .await;
        }
    });

    let status = child.wait().await.context("ffmpeg process error")?;
    let _ = progress_task.await;

    if !status.success() {
        return Err(anyhow!("ffmpeg exited with status {}", status));
    }

    Ok(h265_path)
}
