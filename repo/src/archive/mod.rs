use crate::app::AppEvent;
use crate::state::State;
use crate::telegram::TelegramClient;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};

const DEFAULT_CONCURRENCY: usize = 3;

pub fn start_archive_run(
    state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) {
    tokio::spawn(async move {
        if let Err(e) = run_archive_loop(state, telegram_client, tx.clone(), pause_flag).await {
            let _ = tx.send(AppEvent::ArchiveError(e.to_string())).await;
        } else {
            let _ = tx.send(AppEvent::ArchiveComplete).await;
        }
    });
}

async fn run_archive_loop(
    state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    pause_flag: Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    let source_channel_id = state
        .source_channel_id
        .ok_or_else(|| anyhow::anyhow!("Source channel not set"))?;

    // We need the resolved peer to iterate messages
    // To borrow appropriately without holding lock across await,
    let input_peer = match telegram_client.get_input_peer(source_channel_id).await {
        Some(peer) => peer,
        None => anyhow::bail!(
            "Channel ID not found in memory cache. Please resolve the channel username again."
        ),
    };

    let semaphore = Arc::new(Semaphore::new(DEFAULT_CONCURRENCY));

    // Keep a cursor of the lowest message ID processed so far (since grammers retrieves newest-first)
    let mut lowest_msg_id = state.message_cursor.unwrap_or(i32::MAX);
    let mut local_messages_processed = 0;

    // Create the message iterator
    let mut message_iter = telegram_client.client.iter_messages(input_peer.clone());
    if let Some(cursor) = state.message_cursor {
        message_iter = message_iter.offset_id(cursor);
    }

    // To respect the chunking delay
    while let Some(msg) = crate::retry_flood_wait!(message_iter.next())? {
        while pause_flag.load(std::sync::atomic::Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        let msg_id = msg.id();

        // Update lowest msg id processed in this batch
        if msg_id < lowest_msg_id {
            lowest_msg_id = msg_id;
        }

        // Apply filtering logic
        if should_download(&msg, &state.filters) {
            // Check if already processed
            let status = state.download_status.get(&msg_id);
            let mut needs_download = true;
            let mut existing_caption = None;

            match status {
                Some(crate::state::DownloadStatus::Uploaded) => needs_download = false,
                Some(crate::state::DownloadStatus::Complete { caption }) => {
                    needs_download = false;
                    existing_caption = caption.clone();
                }
                Some(crate::state::DownloadStatus::Skipped) => needs_download = false,
                _ => {}
            }

            let dest_group_id = state.dest_group_id;
            let dest_topic_id = state.dest_topic_id;
            let needs_upload = dest_group_id.is_some()
                && !matches!(
                    status,
                    Some(crate::state::DownloadStatus::Uploaded)
                        | Some(crate::state::DownloadStatus::Skipped)
                );

            if needs_download || needs_upload {
                // Spawn worker
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let tx_clone = tx.clone();
                let telegram_client_clone = Arc::clone(&telegram_client);
                let local_download_path = state.local_download_path.clone();
                let media = msg.media();
                let include_descriptions = state.filters.include_text_descriptions;
                let msg_text = msg.text().to_string();
                let input_peer_clone = input_peer.clone();

                tokio::spawn(async move {
                    let mut filename = msg_id.to_string();
                    let mut is_photo = false;

                    if let Some(grammers_client::media::Media::Document(doc)) = &media {
                        if let Some(name) = doc.name() {
                            filename = format!("{}_{}", msg_id, name);
                        } else if let Some(mime) = doc.mime_type() {
                            let ext = mime.split('/').next_back().unwrap_or("bin");
                            filename = format!("{}.{}", msg_id, ext);
                        }
                    } else if let Some(grammers_client::media::Media::Photo(_)) = &media {
                        filename = format!("{}.jpg", msg_id);
                        is_photo = true;
                    }

                    let file_path = format!("{}/{}", local_download_path, filename);

                    let _ = tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                        msg_id,
                        status: crate::state::DownloadStatus::InProgress { bytes_received: 0 },
                    });

                    let mut file = match tokio::fs::File::create(&file_path).await {
                        Ok(f) => f,
                        Err(e) => {
                            let _ = tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                                msg_id,
                                status: crate::state::DownloadStatus::Failed {
                                    reason: e.to_string(),
                                },
                            });
                            return;
                        }
                    };

                    let mut bytes_received = 0u64;
                    use tokio::io::AsyncWriteExt;

                    if is_photo {
                        if let Some(grammers_client::media::Media::Photo(photo)) = &media {
                            let mut download_iter =
                                telegram_client_clone.client.iter_download(photo);
                            loop {
                                match crate::retry_flood_wait!(download_iter.next()) {
                                    Ok(Some(chunk)) => {
                                        if let Err(e) = file.write_all(&chunk).await {
                                            let _ = tx_clone.try_send(
                                                crate::app::AppEvent::DownloadProgress {
                                                    msg_id,
                                                    status: crate::state::DownloadStatus::Failed {
                                                        reason: e.to_string(),
                                                    },
                                                },
                                            );
                                            return;
                                        }
                                        bytes_received += chunk.len() as u64;
                                        let _ = tx_clone.try_send(
                                            crate::app::AppEvent::DownloadProgress {
                                                msg_id,
                                                status: crate::state::DownloadStatus::InProgress {
                                                    bytes_received,
                                                },
                                            },
                                        );
                                    }
                                    Ok(None) => break,
                                    Err(e) => {
                                        let _ = tx_clone.try_send(
                                            crate::app::AppEvent::DownloadProgress {
                                                msg_id,
                                                status: crate::state::DownloadStatus::Failed {
                                                    reason: e.to_string(),
                                                },
                                            },
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                    } else if let Some(grammers_client::media::Media::Document(doc)) = &media {
                        let mut download_iter = telegram_client_clone.client.iter_download(doc);
                        loop {
                            match crate::retry_flood_wait!(download_iter.next()) {
                                Ok(Some(chunk)) => {
                                    if let Err(e) = file.write_all(&chunk).await {
                                        let _ = tx_clone.try_send(
                                            crate::app::AppEvent::DownloadProgress {
                                                msg_id,
                                                status: crate::state::DownloadStatus::Failed {
                                                    reason: e.to_string(),
                                                },
                                            },
                                        );
                                        return;
                                    }
                                    bytes_received += chunk.len() as u64;
                                    let _ =
                                        tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                                            msg_id,
                                            status: crate::state::DownloadStatus::InProgress {
                                                bytes_received,
                                            },
                                        });
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    let _ =
                                        tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                                            msg_id,
                                            status: crate::state::DownloadStatus::Failed {
                                                reason: e.to_string(),
                                            },
                                        });
                                    return;
                                }
                            }
                        }
                    }

                    let mut description = existing_caption;

                    if needs_download {
                        if include_descriptions {
                            match telegram_client_clone
                                .get_media_description(&input_peer_clone, msg_id, &msg_text)
                                .await
                            {
                                Ok(Some(desc)) => {
                                    let dot_idx = filename.rfind('.').unwrap_or(filename.len());
                                    let txt_filename = format!("{}.txt", &filename[..dot_idx]);
                                    let txt_path =
                                        format!("{}/{}", local_download_path, txt_filename);
                                    if let Err(e) = tokio::fs::write(&txt_path, &desc).await {
                                        let _ = tx_clone.try_send(
                                            crate::app::AppEvent::ArchiveError(format!(
                                                "Failed to save description for {}: {}",
                                                msg_id, e
                                            )),
                                        );
                                    }
                                    description = Some(desc);
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    let _ = tx_clone.try_send(crate::app::AppEvent::ArchiveError(
                                        format!(
                                            "Failed to fetch description for {}: {}",
                                            msg_id, e
                                        ),
                                    ));
                                }
                            }
                        }

                        let _ = tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                            msg_id,
                            status: crate::state::DownloadStatus::Complete {
                                caption: description.clone(),
                            },
                        });
                    }

                    if needs_upload && let Some(group_id) = dest_group_id {
                        if let Some(dest_peer) =
                            telegram_client_clone.get_input_peer(group_id).await
                        {
                            match telegram_client_clone
                                .upload_media(
                                    &file_path,
                                    &dest_peer,
                                    dest_topic_id,
                                    description,
                                    is_photo,
                                )
                                .await
                            {
                                Ok(_) => {
                                    let _ =
                                        tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                                            msg_id,
                                            status: crate::state::DownloadStatus::Uploaded,
                                        });
                                }
                                Err(e) => {
                                    let _ =
                                        tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                                            msg_id,
                                            status: crate::state::DownloadStatus::Failed {
                                                reason: e.to_string(),
                                            },
                                        });
                                }
                            }
                        } else {
                            let _ = tx_clone.try_send(crate::app::AppEvent::DownloadProgress {
                                msg_id,
                                status: crate::state::DownloadStatus::Failed {
                                    reason: "Destination peer not found in cache".into(),
                                },
                            });
                        }
                    }

                    drop(permit);
                });
            }
        }

        local_messages_processed += 1;
        if local_messages_processed >= 100 {
            // We've processed a chunk of 100 messages locally.
            // Persist the cursor
            // Since `state` was moved into the function, we can't easily mutate and save without having a handle,
            // but we can send an event to App to save state.
            let _ = tx.send(AppEvent::SaveCursor(lowest_msg_id)).await;

            // Apply 500ms delay
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            local_messages_processed = 0;
        }
    }

    // End of stream, save final cursor
    let _ = tx.send(AppEvent::SaveCursor(lowest_msg_id)).await;

    // Wait for all workers to finish
    // A simple way is to acquire *all* permits
    let _ = semaphore
        .acquire_many(DEFAULT_CONCURRENCY as u32)
        .await
        .unwrap();

    Ok(())
}

fn should_download(
    msg: &grammers_client::message::Message,
    filters: &crate::state::Filters,
) -> bool {
    let media = match msg.media() {
        Some(m) => m,
        None => return false,
    };

    match media {
        grammers_client::media::Media::Photo(_) => {
            if !filters.filter_image {
                return false;
            }
            true
        }
        // In the rare case of direct Media::Video/Audio usage by grammers if exposed
        // Note: in 0.9.0 grammers_client Media might not expose Video directly but as Document,
        // but to strictly follow the rules mappings we check if it was somehow mapped or exists.
        // Actually, we'll implement it strictly as requested.
        _ => {
            // Check for other variants or fallback to document logic
            if let grammers_client::media::Media::Document(doc) = media {
                let mime = doc.mime_type().unwrap_or("");
                let size = doc.size();

                if let Some(s) = size
                    && (s as u64) < filters.min_size_bytes
                {
                    return false;
                }

                if mime.starts_with("video/") && filters.filter_video {
                    return true;
                }
                if mime.starts_with("audio/") && filters.filter_audio {
                    return true;
                }
                if mime.starts_with("image/") && filters.filter_image {
                    return true;
                }

                let is_archive = mime == "application/zip"
                    || mime == "application/x-rar-compressed"
                    || mime == "application/x-7z-compressed"
                    || mime == "application/gzip"
                    || mime == "application/x-tar";

                if is_archive && filters.filter_archive {
                    return true;
                }
                false
            } else {
                false
            }
        }
    }
}
