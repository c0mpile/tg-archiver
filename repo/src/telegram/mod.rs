use anyhow::{Context, Result};
use grammers_client::{Client, SenderPool, SignInError};
use grammers_session::storages::SqliteSession;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

type TopicList = Vec<(i32, String)>;
type DialogListCache = Arc<RwLock<Option<Vec<(i64, String)>>>>;

pub struct TelegramClient {
    pub client: Client,
    // We hold onto the pool task so it gets aborted if/when TelegramClient is dropped,
    // though in this app it typically runs for the lifetime of the program.
    _pool_task: tokio::task::JoinHandle<()>,

    channel_list_cache: DialogListCache,
    group_list_cache: DialogListCache,
    topic_cache: Arc<RwLock<HashMap<i64, TopicList>>>,
    peer_cache: Arc<RwLock<HashMap<i64, grammers_tl_types::enums::InputPeer>>>,
}

impl TelegramClient {
    pub async fn init() -> Result<Self> {
        let api_id = std::env::var("TG_API_ID")
            .context("TG_API_ID must be set")?
            .parse::<i32>()
            .context("TG_API_ID must be an integer")?;
        let api_hash = std::env::var("TG_API_HASH").context("TG_API_HASH must be set")?;
        let session_file =
            std::env::var("TG_SESSION_FILE").context("TG_SESSION_FILE must be set")?;

        let session = Arc::new(
            SqliteSession::open(&session_file)
                .await
                .context("Failed to open session file")?,
        );

        // Intialize SenderPool
        let SenderPool { runner, handle, .. } = SenderPool::new(Arc::clone(&session), api_id);

        let client = Client::new(handle);
        let pool_task = tokio::spawn(runner.run());

        if !client.is_authorized().await? {
            println!("Telegram session is not authorized. Starting interactive authentication...");

            let phone = match std::env::var("TG_PHONE") {
                Ok(p) => p,
                Err(_) => {
                    print!("Enter your phone number (e.g., +1234567890): ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().lock().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };

            let token = client
                .request_login_code(&phone, &api_hash)
                .await
                .context("Failed to request login code")?;

            print!("Enter the authorization code sent to your Telegram: ");
            io::stdout().flush()?;
            let mut code = String::new();
            io::stdin().lock().read_line(&mut code)?;
            let code = code.trim();

            match client.sign_in(&token, code).await {
                Ok(_) => {
                    println!("Authentication successful!");
                }
                Err(SignInError::PasswordRequired(password_token)) => {
                    print!("Enter your 2FA password: ");
                    io::stdout().flush()?;
                    let mut password = String::new();
                    io::stdin().lock().read_line(&mut password)?;
                    let password = password.trim();

                    client
                        .check_password(password_token, password)
                        .await
                        .context("Failed to authenticate with 2FA password")?;
                    println!("2FA Authentication successful!");
                }
                Err(e) => {
                    anyhow::bail!("Failed to sign in: {}", e);
                }
            }

            // In grammers v0.9, SqliteSession saves automatically most of the time,
            // but we can ensure it's saved.
            // Client session is updated automatically through the arc reference.
        }

        Ok(Self {
            client,
            _pool_task: pool_task,
            channel_list_cache: Arc::new(RwLock::new(None)),
            group_list_cache: Arc::new(RwLock::new(None)),
            topic_cache: Arc::new(RwLock::new(HashMap::new())),
            peer_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn get_input_peer(
        &self,
        peer_id: i64,
    ) -> Option<grammers_tl_types::enums::InputPeer> {
        self.peer_cache.read().await.get(&peer_id).cloned()
    }

    pub async fn list_topics(&self, group_id: i64) -> Result<Vec<(i32, String)>> {
        {
            let cache = self.topic_cache.read().await;
            if let Some(cached) = cache.get(&group_id) {
                return Ok(cached.clone());
            }
        }

        let peer_cache = self.peer_cache.read().await;
        let input_peer = peer_cache
            .get(&group_id)
            .context(
                "Group ID not found in memory cache. Please resolve the group username first.",
            )?
            .clone();
        drop(peer_cache);

        let mut topics = Vec::new();

        // Fetch topics using raw TL function if it's a channel/megagroup
        let limit = 100;
        let offset_date = 0;
        let offset_id = 0;
        let mut offset_topic = 0;

        loop {
            let req = grammers_tl_types::functions::messages::GetForumTopics {
                peer: input_peer.clone(),
                q: None,
                offset_date,
                offset_id,
                offset_topic,
                limit,
            };

            let res = crate::retry_flood_wait!(self.client.invoke(&req));
            let res = match res {
                Ok(r) => r,
                Err(_) => break, // If error (e.g., chat is not a forum), just break
            };

            let mut batch_count = 0;
            match res {
                grammers_tl_types::enums::messages::ForumTopics::Topics(topics_data) => {
                    for topic in topics_data.topics {
                        if let grammers_tl_types::enums::ForumTopic::Topic(t) = topic {
                            topics.push((t.id, t.title));
                            offset_topic = t.id; // Usually we use offset parameters to paginate
                            batch_count += 1;
                        }
                    }
                }
            }

            if batch_count < limit {
                break;
            }
        }

        self.topic_cache
            .write()
            .await
            .insert(group_id, topics.clone());
        Ok(topics)
    }

    pub async fn get_joined_channels(&self) -> Result<Vec<(i64, String)>> {
        {
            let cache = self.channel_list_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                return Ok(cached.clone());
            }
        }

        let mut channels = Vec::new();
        let mut new_peers = Vec::new();
        let mut iter = self.client.iter_dialogs();

        while let Some(dialog) = crate::retry_flood_wait!(iter.next())? {
            let peer = dialog.peer().clone();
            if let grammers_client::peer::Peer::Channel(ref c) = peer
                && c.raw.broadcast
            {
                let id = peer.id().bot_api_dialog_id();
                let title = peer.name().unwrap_or("Unknown").to_string();
                let input_peer_opt: Option<grammers_tl_types::enums::InputPeer> =
                    peer.to_ref().await.map(|r| r.into());
                if let Some(peer) = input_peer_opt {
                    new_peers.push((id, peer));
                }
                channels.push((id, title));
            }
        }

        if !new_peers.is_empty() {
            let mut cache = self.peer_cache.write().await;
            for (id, peer) in new_peers {
                cache.insert(id, peer);
            }
        }

        *self.channel_list_cache.write().await = Some(channels);
        Ok(self
            .channel_list_cache
            .read()
            .await
            .as_ref()
            .unwrap()
            .clone())
    }

    pub async fn get_joined_groups(&self) -> Result<Vec<(i64, String)>> {
        {
            let cache = self.group_list_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                return Ok(cached.clone());
            }
        }

        let mut groups = Vec::new();
        let mut new_peers = Vec::new();
        let mut iter = self.client.iter_dialogs();

        while let Some(dialog) = crate::retry_flood_wait!(iter.next())? {
            let peer = dialog.peer().clone();
            let is_target = match &peer {
                grammers_client::peer::Peer::Group(_) => true,
                grammers_client::peer::Peer::Channel(c) if c.raw.megagroup => true,
                _ => false,
            };

            if is_target {
                let id = peer.id().bot_api_dialog_id();
                let title = peer.name().unwrap_or("Unknown").to_string();
                let input_peer_opt: Option<grammers_tl_types::enums::InputPeer> =
                    peer.to_ref().await.map(|r| r.into());
                if let Some(peer) = input_peer_opt {
                    new_peers.push((id, peer));
                }
                groups.push((id, title));
            }
        }

        if !new_peers.is_empty() {
            let mut cache = self.peer_cache.write().await;
            for (id, peer) in new_peers {
                cache.insert(id, peer);
            }
        }

        *self.group_list_cache.write().await = Some(groups);
        Ok(self.group_list_cache.read().await.as_ref().unwrap().clone())
    }

    pub async fn create_topic(&self, group_id: i64, title: &str) -> Result<i32> {
        let peer_cache = self.peer_cache.read().await;
        let input_peer = peer_cache
            .get(&group_id)
            .context("Group ID not found in memory cache.")?
            .clone();
        drop(peer_cache);

        use grammers_tl_types::functions::messages::CreateForumTopic;

        // rand::random is okay if available, but let's just use a simple mock random id or timestamp
        let random_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        let req = CreateForumTopic {
            title_missing: false,
            peer: input_peer,
            title: title.to_string(),
            icon_color: None,
            icon_emoji_id: None,
            random_id,
            send_as: None,
        };

        let res = crate::retry_flood_wait!(self.client.invoke(&req))?;

        // Updates returned by CreateForumTopic contains the new topic ID
        // Usually, the easiest way is to parse the updates
        let topic_id = match res {
            grammers_tl_types::enums::Updates::Updates(u) => {
                let mut found_id = None;
                for update in u.updates {
                    if let grammers_tl_types::enums::Update::MessageId(m) = update {
                        found_id = Some(m.id);
                        break;
                    }
                }
                found_id
            }
            _ => None,
        };

        if let Some(tid) = topic_id {
            Ok(tid)
        } else {
            // Fallback: list topics and find the one with this name
            let topics = self.list_topics(group_id).await?;
            topics
                .into_iter()
                .find(|(_, t)| t == title)
                .map(|(id, _)| id)
                .context("Failed to extract new topic ID from response")
        }
    }

    pub async fn forward_messages_as_copy(
        &self,
        from_peer: &grammers_tl_types::enums::InputPeer,
        to_peer: &grammers_tl_types::enums::InputPeer,
        msg_ids: &[i32],
        topic_id: Option<i32>,
    ) -> Result<()> {
        use grammers_tl_types::functions::messages::ForwardMessages;
        let random_id: Vec<i64> = msg_ids
            .iter()
            .map(|_| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as i64
            })
            .collect();

        let req = ForwardMessages {
            silent: false,
            background: false,
            with_my_score: false,
            drop_author: true,
            drop_media_captions: false,
            noforwards: false,
            allow_paid_floodskip: false,
            from_peer: from_peer.clone(),
            id: msg_ids.to_vec(),
            random_id,
            to_peer: to_peer.clone(),
            top_msg_id: topic_id,
            reply_to: None,
            schedule_date: None,
            schedule_repeat_period: None,
            send_as: None,
            quick_reply_shortcut: None,
            effect: None,
            video_timestamp: None,
            allow_paid_stars: None,
            suggested_post: None,
        };

        let _res = crate::retry_flood_wait!(self.client.invoke(&req))
            .context("Failed to forward messages as copy")?;

        Ok(())
    }
}

/// Helper macro to handle `FloodWait` errors by retrying once.
/// It catches `grammers_mtsender::InvocationError::Rpc` with `FLOOD_WAIT_X`.
#[macro_export]
macro_rules! retry_flood_wait {
    ($client_call:expr) => {{
        let mut retried = false;
        loop {
            match $client_call.await {
                Ok(val) => break Ok(val),
                Err(e) => {
                    let err = anyhow::anyhow!(e);
                    let mut is_flood = false;
                    let mut wait_sec = 0;
                    if let Some(grammers_mtsender::InvocationError::Rpc(rpc_error)) =
                        err.downcast_ref::<grammers_mtsender::InvocationError>()
                    {
                        if rpc_error.name == "FLOOD_WAIT" {
                            is_flood = true;
                            wait_sec = rpc_error.value.unwrap_or(0) as u64;
                        }
                    } else if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                        if let Some(inner) = io_err.get_ref() {
                            if let Some(grammers_mtsender::InvocationError::Rpc(rpc_error)) =
                                inner.downcast_ref::<grammers_mtsender::InvocationError>()
                            {
                                if rpc_error.name == "FLOOD_WAIT" {
                                    is_flood = true;
                                    wait_sec = rpc_error.value.unwrap_or(0) as u64;
                                }
                            }
                        }
                    }

                    if is_flood {
                        if retried {
                            break Err(anyhow::anyhow!($crate::error::AppError::FloodWait(
                                std::time::Duration::from_secs(wait_sec)
                            )));
                        }
                        let delay = wait_sec + 2;
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                        retried = true;
                        continue;
                    }

                    break Err(err);
                }
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    // requires: TG_API_ID, TG_API_HASH, TG_SESSION_FILE
    async fn test_init_telegram_client() {
        dotenvy::dotenv().ok();
        let client = TelegramClient::init().await;
        assert!(
            client.is_ok(),
            "Failed to init Telegram client: {:?}",
            client.err()
        );
    }
}
