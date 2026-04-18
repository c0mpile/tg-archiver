use crate::app::AppEvent;
use crate::archive::run_archive_loop;
use crate::state::State;
use crate::telegram::TelegramClient;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

pub fn start_monitoring_loop(
    state: State,
    telegram_client: Arc<TelegramClient>,
    tx: mpsc::Sender<AppEvent>,
    mut cancel_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let interval_secs = state.poll_interval_secs.max(60);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        // Create a dummy pause flag that is always false for the background run.
        let pause_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let _ = tx.send(AppEvent::MonitoringTick).await;

                    // Iterate sequentially
                    for i in 0..state.channel_pairs.len() {
                        // Check for cancellation before processing each pair
                        if *cancel_rx.borrow() {
                            return;
                        }

                        let _ = tx.send(AppEvent::PairSyncStarted { pair_index: i }).await;

                        // Background runs suppress SaveCursor and ArchiveTotalCount etc.
                        match run_archive_loop(
                            state.clone(),
                            i,
                            Arc::clone(&telegram_client),
                            tx.clone(),
                            Arc::clone(&pause_flag),
                            true,
                        )
                        .await
                        {
                            Ok(Some(last_id)) => {
                                let _ = tx.send(AppEvent::PairSynced {
                                    pair_index: i,
                                    last_forwarded_message_id: last_id,
                                }).await;
                            }
                            Ok(None) => {
                                // No messages forwarded yet and none in state, ignore.
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::PairError {
                                    pair_index: i,
                                    error: e.to_string(),
                                }).await;
                            }
                        }
                    }
                }
                _ = cancel_rx.changed() => {
                    if *cancel_rx.borrow() {
                        break;
                    }
                }
            }
        }
    });
}
