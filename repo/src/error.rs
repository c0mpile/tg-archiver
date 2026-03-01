// Primary error type for the tg-archiver application
#[derive(thiserror::Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Application error: {0}")]
    General(String),
}
