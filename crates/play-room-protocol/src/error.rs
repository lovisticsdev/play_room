use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("json serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("message line is empty")]
    EmptyLine,
}
