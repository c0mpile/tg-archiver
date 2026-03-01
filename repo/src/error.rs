// Primary error type for the tg-archiver application
#[derive(thiserror::Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Application error: {0}")]
    General(String),

    #[error("Telegram FloodWait for {0:?}")]
    FloodWait(std::time::Duration),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Session expired")]
    SessionExpired,
}
