use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("Audio error: {0}")]
    Audio(String),
    #[error("AI error: {0}")]
    Ai(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("{0}")]
    Generic(String),
}
