use play_room_protocol::ProtocolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("invalid command: {0}")]
    InvalidCommand(String),
}
