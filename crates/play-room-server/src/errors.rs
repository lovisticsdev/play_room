use play_room_core::CoreError;
use play_room_protocol::ProtocolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("core error: {0}")]
    Core(#[from] CoreError),

    #[error("config error: {0}")]
    Config(String),
}
