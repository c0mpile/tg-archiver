use anyhow::{Context, Result};
use grammers_client::{Client, SignInError, SenderPool};
use grammers_session::storages::SqliteSession;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

#[allow(dead_code)]
pub struct TelegramClient {
    pub client: Client,
    // We hold onto the pool task so it gets aborted if/when TelegramClient is dropped, 
    // though in this app it typically runs for the lifetime of the program.
    _pool_task: tokio::task::JoinHandle<()>,
}

impl TelegramClient {
    pub async fn init() -> Result<Self> {
        let api_id = std::env::var("TG_API_ID")
            .context("TG_API_ID must be set")?
            .parse::<i32>()
            .context("TG_API_ID must be an integer")?;
        let api_hash = std::env::var("TG_API_HASH")
            .context("TG_API_HASH must be set")?;
        let session_file = std::env::var("TG_SESSION_FILE")
            .context("TG_SESSION_FILE must be set")?;

        let session = Arc::new(SqliteSession::open(&session_file).await.context("Failed to open session file")?);

        // Intialize SenderPool
        let SenderPool {
            runner,
            handle,
            ..
        } = SenderPool::new(Arc::clone(&session), api_id);

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
        })
    }
}

/// Helper macro to handle `FloodWait` errors by retrying once.
/// It catches `grammers_mtsender::InvocationError::Rpc` with `FLOOD_WAIT_X`.
#[macro_export]
macro_rules! retry_flood_wait {
    ($client_call:expr) => {
        {
            let mut retried = false;
            loop {
                match $client_call.await {
                    Ok(val) => break Ok(val),
                    Err(e) => {
                        if let grammers_mtsender::InvocationError::Rpc(rpc_error) = &e {
                            if rpc_error.name == "FLOOD_WAIT" {
                                if retried {
                                    break Err($crate::error::AppError::FloodWait(
                                        std::time::Duration::from_secs(rpc_error.value.unwrap_or(0) as u64),
                                    ).into());
                                }
                                let wait_seconds = rpc_error.value.unwrap_or(0) as u64;
                                let delay = wait_seconds + 2; // adding 2 seconds buffer
                                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                                retried = true;
                                continue;
                            }
                        }
                        // For any other error, return it
                        break Err(anyhow::anyhow!(e));
                    }
                }
            }
        }
    }
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
        assert!(client.is_ok(), "Failed to init Telegram client: {:?}", client.err());
    }
}
