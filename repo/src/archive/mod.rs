use crate::app::AppEvent;
use crate::state::State;
use crate::telegram::TelegramClient;
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn start_archive_run(
    state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) {
    tokio::spawn(async move {
        if let Err(e) = run_archive_loop(state, telegram_client, tx.clone(), pause_flag).await {
            let _ = tx.send(AppEvent::ArchiveLog(format!("Error: {}", e))).await;
            let _ = tx.send(AppEvent::ArchiveError(e.to_string())).await;
        } else {
            let _ = tx
                .send(AppEvent::ArchiveLog(
                    "✓ Archive complete. Channel is up to date.".to_string(),
                ))
                .await;
            let _ = tx.send(AppEvent::ArchiveComplete).await;
        }
    });
}

async fn run_archive_loop(
    mut state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    let source_channel_id = state
        .source_channel_id
        .ok_or_else(|| anyhow::anyhow!("Source channel not set"))?;

    let dest_group_id = state
        .dest_group_id
        .ok_or_else(|| anyhow::anyhow!("Destination group not set"))?;

    let input_peer_source = match telegram_client.get_input_peer(source_channel_id).await {
        Some(peer) => peer,
        None => anyhow::bail!("Source channel ID not found in memory cache. Please resolve again."),
    };

    let input_peer_dest = match telegram_client.get_input_peer(dest_group_id).await {
        Some(peer) => peer,
        None => anyhow::bail!("Destination group ID not found in memory cache."),
    };

    // Step 1: Find highest message ID dynamically
    // A simple iter_messages with limit 1 gets the latest message
    let mut highest_msg_id = 1;
    let mut iter = telegram_client
        .client
        .iter_messages(input_peer_source.clone())
        .limit(1);

    if let Some(msg) = crate::retry_flood_wait!(iter.next())? {
        highest_msg_id = msg.id();
    }

    // Check if we need to apply post count threshold
    if state.post_count_threshold > 0 {
        let lowest_allowed = highest_msg_id - (state.post_count_threshold as i32) + 1;
        if state.last_forwarded_message_id.unwrap_or(0) < lowest_allowed {
            state.last_forwarded_message_id = Some(lowest_allowed - 1);
        }
    }

    // Determine starting point
    let start_id = match state.last_forwarded_message_id {
        Some(id) => id + 1,
        None => 1,
    };

    let _ = tx
        .send(AppEvent::ArchiveStarted {
            start_id,
            highest_msg_id,
        })
        .await;
    let _ = tx
        .send(AppEvent::ArchiveLog(format!(
            "Starting archive: messages {} → {}",
            start_id, highest_msg_id
        )))
        .await;

    if start_id > highest_msg_id {
        return Ok(()); // Nothing to do
    }

    // Step 2: Iterate in chunks of 100
    let chunk_size = 100;
    let mut current_start = start_id;

    while current_start <= highest_msg_id {
        while pause_flag.load(std::sync::atomic::Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        let mut current_end = current_start + chunk_size - 1;
        if current_end > highest_msg_id {
            current_end = highest_msg_id;
        }

        let ids: Vec<i32> = (current_start..=current_end).collect();

        // Fetch messages by ID
        let messages = crate::retry_flood_wait!(
            telegram_client
                .client
                .get_messages_by_id(input_peer_source.clone(), &ids),
            Some(tx.clone())
        )?;

        let mut valid_msg_ids = Vec::new();

        for msg in messages.into_iter().flatten() {
            // Filter out service messages and empty messages
            // Usually service messages have neither text nor media
            if msg.text().trim().is_empty() && msg.media().is_none() {
                continue;
            }
            valid_msg_ids.push(msg.id());
        }

        if !valid_msg_ids.is_empty() {
            // Forward the valid messages
            telegram_client
                .forward_messages_as_copy(
                    &input_peer_source,
                    &input_peer_dest,
                    &valid_msg_ids,
                    state.dest_topic_id,
                    Some(tx.clone()),
                )
                .await?;
            let _ = tx
                .send(AppEvent::ArchiveLog(format!(
                    "Forwarded {} messages (IDs {}–{})",
                    valid_msg_ids.len(),
                    current_start,
                    current_end
                )))
                .await;
        } else {
            let _ = tx
                .send(AppEvent::ArchiveLog(format!(
                    "Skipped chunk {}–{} (no valid messages)",
                    current_start, current_end
                )))
                .await;
        }

        // Update state and UI
        state.last_forwarded_message_id = Some(current_end);
        let _ = tx.send(AppEvent::SaveCursor(current_end)).await;

        // Apply delay between chunks
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        current_start = current_end + 1;
    }

    Ok(())
}
