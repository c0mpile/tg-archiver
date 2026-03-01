use anyhow::{Context, Result};
use grammers_client::{Client, SenderPool, SignInError};
use grammers_session::storages::SqliteSession;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

type TopicList = Vec<(i32, String)>;

pub struct TelegramClient {
    pub client: Client,
    // We hold onto the pool task so it gets aborted if/when TelegramClient is dropped,
    // though in this app it typically runs for the lifetime of the program.
    _pool_task: tokio::task::JoinHandle<()>,

    // App caches
    channel_cache: Arc<RwLock<HashMap<String, (i64, String)>>>,
    group_cache: Arc<RwLock<HashMap<String, (i64, String)>>>,
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
            channel_cache: Arc::new(RwLock::new(HashMap::new())),
            group_cache: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn resolve_channel(&self, username: &str) -> Result<(i64, String)> {
        let username_clean = username.trim_start_matches('@').to_string();

        // Check cache
        {
            let cache = self.channel_cache.read().await;
            if let Some(cached) = cache.get(&username_clean) {
                return Ok(cached.clone());
            }
        }

        // Resolve via API
        let chat = crate::retry_flood_wait!(self.client.resolve_username(&username_clean))?
            .context("Username not found")?;

        let id: i64 = chat.id().bot_api_dialog_id();

        let title = chat.name().unwrap_or("Unknown").to_string();

        let input_peer_opt: Option<grammers_tl_types::enums::InputPeer> =
            chat.to_ref().await.map(|r| r.into());
        if let Some(peer) = input_peer_opt {
            self.peer_cache.write().await.insert(id, peer);
        }

        let result = (id, title);
        self.channel_cache
            .write()
            .await
            .insert(username_clean, result.clone());
        Ok(result)
    }

    pub async fn resolve_group(&self, username: &str) -> Result<(i64, String)> {
        let username_clean = username.trim_start_matches('@').to_string();

        {
            let cache = self.group_cache.read().await;
            if let Some(cached) = cache.get(&username_clean) {
                return Ok(cached.clone());
            }
        }

        let chat = crate::retry_flood_wait!(self.client.resolve_username(&username_clean))?
            .context("Group username not found")?;

        let id: i64 = chat.id().bot_api_dialog_id();
        let title = chat.name().unwrap_or("Unknown").to_string();

        let input_peer_opt: Option<grammers_tl_types::enums::InputPeer> =
            chat.to_ref().await.map(|r| r.into());
        if let Some(peer) = input_peer_opt {
            self.peer_cache.write().await.insert(id, peer);
        }

        let result = (id, title);
        self.group_cache
            .write()
            .await
            .insert(username_clean, result.clone());
        Ok(result)
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

    pub async fn get_media_description(
        &self,
        peer: &grammers_tl_types::enums::InputPeer,
        msg_id: i32,
        msg_text: &str,
    ) -> Result<Option<String>> {
        if !msg_text.trim().is_empty() {
            return Ok(Some(msg_text.trim().to_string()));
        }

        let mut count = 0;
        let mut iter = self.client.iter_messages(peer.clone()).offset_id(msg_id);

        while let Some(prev_msg) = crate::retry_flood_wait!(iter.next())? {
            count += 1;
            if count > 3 {
                break;
            }
            if prev_msg.media().is_none() {
                let txt = prev_msg.text();
                if !txt.trim().is_empty() {
                    return Ok(Some(txt.trim().to_string()));
                }
            }
        }

        Ok(None)
    }

    pub async fn upload_media(
        &self,
        file_path: &str,
        peer: &grammers_tl_types::enums::InputPeer,
        topic_id: Option<i32>,
        caption: Option<String>,
        is_photo: bool,
    ) -> Result<()> {
        let uploaded = crate::retry_flood_wait!(self.client.upload_file(file_path))
            .context("Failed to upload file")?;

        let mut msg = grammers_client::message::InputMessage::new();
        if let Some(text) = caption
            && !text.trim().is_empty()
        {
            msg = msg.text(text);
        }

        if is_photo {
            msg = msg.photo(uploaded);
        } else {
            msg = msg.document(uploaded);
        }

        if let Some(tid) = topic_id {
            msg = msg.reply_to(Some(tid));
        }

        crate::retry_flood_wait!(self.client.send_message(peer.clone(), msg.clone()))
            .context("Failed to send uploaded media to topic")?;

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
