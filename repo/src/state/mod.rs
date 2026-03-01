use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct State {
    #[serde(default)]
    pub source_channel_id: Option<i64>,
    #[serde(default)]
    pub source_channel_title: Option<String>,
    #[serde(default)]
    pub dest_group_id: Option<i64>,
    #[serde(default)]
    pub dest_topic_id: Option<i32>,
    #[serde(default)]
    pub dest_group_title: Option<String>,
    #[serde(default)]
    pub dest_topic_title: Option<String>,
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub download_status: HashMap<i32, DownloadStatus>, // message ID to status
    #[serde(default)]
    pub message_cursor: Option<i32>,
    #[serde(default)]
    pub local_download_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Filters {
    #[serde(default = "default_true")]
    pub filter_video: bool,
    #[serde(default = "default_true")]
    pub filter_audio: bool,
    #[serde(default = "default_true")]
    pub filter_image: bool,
    #[serde(default = "default_true")]
    pub filter_archive: bool,
    #[serde(default)]
    pub min_size_bytes: u64,
    #[serde(default)]
    pub post_count_threshold: u32,
    #[serde(default = "default_true")]
    pub include_text_descriptions: bool,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            filter_video: true,
            filter_audio: true,
            filter_image: true,
            filter_archive: true,
            min_size_bytes: 0,
            post_count_threshold: 0,
            include_text_descriptions: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status")]
pub enum DownloadStatus {
    Pending,
    InProgress { bytes_received: u64 },
    Complete,
    Failed { reason: String },
    Skipped,
}

impl State {
    pub async fn load() -> anyhow::Result<Self> {
        let state_dir = std::env::var("XDG_STATE_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").expect("HOME env var not set");
                std::path::PathBuf::from(home).join(".local/state")
            })
            .join("tg-archiver");

        let state_file = state_dir.join("state.json");

        if !state_file.exists() {
            return Ok(State::default());
        }

        let content = fs::read_to_string(&state_file).await?;
        match serde_json::from_str(&content) {
            Ok(state) => Ok(state),
            Err(e) => {
                anyhow::bail!("State deserialisation failed: {}", e);
            }
        }
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let state_dir = std::env::var("XDG_STATE_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").expect("HOME env var not set");
                std::path::PathBuf::from(home).join(".local/state")
            })
            .join("tg-archiver");

        fs::create_dir_all(&state_dir).await?;

        let state_file = state_dir.join("state.json");
        let tmp_file = state_dir.join("state.json.tmp");

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&tmp_file, content).await?;
        fs::rename(tmp_file, state_file).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut state = State::default();
        state.filters.min_size_bytes = 1000;
        state.download_status.insert(
            123,
            DownloadStatus::InProgress {
                bytes_received: 500,
            },
        );
        state.download_status.insert(
            124,
            DownloadStatus::Failed {
                reason: "timeout".into(),
            },
        );

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: State = serde_json::from_str(&serialized).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_migration_compatibility() {
        let old_json = r#"{
            "source_channel_id": 12345,
            "local_download_path": "/tmp",
            "download_status": {
                "10": { "status": "Pending" },
                "11": { "status": "Failed", "reason": "timeout" }
            }
        }"#;

        let state: State = serde_json::from_str(old_json).unwrap();
        assert_eq!(state.source_channel_id, Some(12345));
        assert_eq!(state.local_download_path, "/tmp");
        assert_eq!(state.filters.filter_video, true);
        assert_eq!(state.filters.min_size_bytes, 0);
        assert_eq!(state.message_cursor, None);
        assert_eq!(
            state.download_status.get(&10),
            Some(&DownloadStatus::Pending)
        );
        assert_eq!(
            state.download_status.get(&11),
            Some(&DownloadStatus::Failed {
                reason: "timeout".to_string()
            })
        );
    }
}
