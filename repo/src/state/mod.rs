use tokio::fs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
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
    pub post_count_threshold: u32,
    #[serde(default)]
    pub last_forwarded_message_id: Option<i32>,
    #[serde(default)]
    pub source_message_count: Option<i32>,
    #[serde(default)]
    pub auto_create_topic: bool,
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
        match serde_json::from_str::<State>(&content) {
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
        state.post_count_threshold = 1000;
        state.last_forwarded_message_id = Some(123);
        state.source_message_count = Some(150);
        state.auto_create_topic = true;

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: State = serde_json::from_str(&serialized).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_migration_compatibility() {
        let old_json = r#"{
            "source_channel_id": 12345,
            "filters": {
                "post_count_threshold": 50
            },
            "download_status": {
                "10": { "status": "Pending" },
                "11": { "status": "Failed", "reason": "timeout" }
            }
        }"#;

        let state: State = serde_json::from_str(old_json).unwrap();
        assert_eq!(state.source_channel_id, Some(12345));
        assert_eq!(state.post_count_threshold, 0); // Since it was moved out of filters and defaults to 0
        assert_eq!(state.last_forwarded_message_id, None);
        assert_eq!(state.auto_create_topic, false);
    }
}
