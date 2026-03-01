// Primary error type for the tg-archiver application
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Application error: {0}")]
    #[allow(dead_code)]
    General(String),

    #[error("Telegram FloodWait for {0:?}")]
    FloodWait(std::time::Duration),

    #[error("Authentication required")]
    #[allow(dead_code)]
    AuthRequired,

    #[error("Session expired")]
    #[allow(dead_code)]
    SessionExpired,
}
