use crate::config::ServerConfig;
use crate::errors::ServerError;
use crate::room_manager::RoomManager;
use crate::session::handle_connection;
use crate::websocket_session::handle_websocket_connection;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{error, info};

const WEBSOCKET_HANDSHAKE_PREFIX: &[u8] = b"GET";

pub async fn run(config: ServerConfig) -> Result<(), ServerError> {
    let listener = TcpListener::bind(config.addr()).await?;
    let manager = Arc::new(Mutex::new(RoomManager::default()));
    info!(addr = %listener.local_addr()?, "play room server listening");

    loop {
        let (stream, addr) = listener.accept().await?;
        let manager = manager.clone();
        tokio::spawn(async move {
            let result = if is_websocket_handshake(&stream).await {
                handle_websocket_connection(stream, manager).await
            } else {
                handle_connection(stream, manager).await
            };

            if let Err(err) = result {
                error!(%addr, ?err, "connection failed");
            }
        });
    }
}

async fn is_websocket_handshake(stream: &TcpStream) -> bool {
    let mut prefix = [0_u8; 3];
    matches!(
        stream.peek(&mut prefix).await,
        Ok(read) if read == prefix.len() && prefix.as_slice() == WEBSOCKET_HANDSHAKE_PREFIX
    )
}
