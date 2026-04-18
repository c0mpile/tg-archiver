use crate::app::{AppEvent, UploadEntry, UploadMode};
use crate::telegram::TelegramClient;
use anyhow::{Context, Result};
use grammers_tl_types::enums::{InputMedia, InputReplyTo};
use grammers_tl_types::functions::messages::SendMedia;
use grammers_tl_types::types::{InputMediaUploadedDocument, InputReplyToMessage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

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
    let uploaded = client
        .client
        .upload_file(local_path)
        .await
        .context("Failed to upload file to Telegram server")?;

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

    let input_peer = client
        .get_input_peer(dest_group_id)
        .await
        .context("Group ID not found in memory cache.")?;

    let media = InputMedia::UploadedDocument(InputMediaUploadedDocument {
        file: uploaded.raw,
        thumb: None,
        mime_type,
        attributes: vec![],
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

        if let Err(e) = upload_file(&client, &path, dest_group_id, dest_topic_id, &caption).await {
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
