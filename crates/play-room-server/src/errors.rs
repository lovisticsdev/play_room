use play_room_core::CoreError;
use play_room_protocol::ProtocolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("websocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("core error: {0}")]
    Core(#[from] CoreError),

    #[error("config error: {0}")]
    Config(String),
}
