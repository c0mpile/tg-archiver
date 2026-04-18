use tokio::fs;

fn default_poll_interval() -> u64 {
    300
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct ChannelPair {
    #[serde(default)]
    pub source_channel_id: Option<i64>,
    pub source_channel_title: String,
    #[serde(default)]
    pub dest_group_id: Option<i64>,
    pub dest_group_title: String,
    #[serde(default)]
    pub dest_topic_id: Option<i32>,
    #[serde(default)]
    pub dest_topic_title: Option<String>,
    #[serde(default)]
    pub last_forwarded_message_id: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct State {
    #[serde(default)]
    pub channel_pairs: Vec<ChannelPair>,
    #[serde(default)]
    pub post_count_threshold: u32,
    #[serde(default)]
    pub auto_create_topic: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            channel_pairs: Vec::new(),
            post_count_threshold: 0,
            auto_create_topic: false,
            poll_interval_secs: 300,
        }
    }
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
        assert_eq!(ChannelPair::default().source_channel_id, None);
        assert_eq!(ChannelPair::default().dest_group_id, None);
        let mut state = State::default();
        assert_eq!(state.poll_interval_secs, 300);
        state.post_count_threshold = 1000;
        state.auto_create_topic = true;
        state.channel_pairs.push(ChannelPair {
            source_channel_id: Some(123),
            source_channel_title: "Source".to_string(),
            dest_group_id: Some(456),
            dest_group_title: "Dest".to_string(),
            dest_topic_id: Some(789),
            dest_topic_title: Some("Topic".to_string()),
            last_forwarded_message_id: Some(42),
        });

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: State = serde_json::from_str(&serialized).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_migration_compatibility() {
        let old_json = r#"{
            "source_channel_id": 12345,
            "source_channel_title": "Old Source",
            "dest_group_id": 67890,
            "last_forwarded_message_id": 42,
            "source_message_count": 100,
            "post_count_threshold": 50,
            "auto_create_topic": true
        }"#;

        let state: State = serde_json::from_str(old_json).unwrap();
        assert_eq!(state.poll_interval_secs, 300);
        assert_eq!(state.post_count_threshold, 50);
        assert_eq!(state.auto_create_topic, true);
        assert!(state.channel_pairs.is_empty());
    }
}
