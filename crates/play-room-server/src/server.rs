use crate::config::ServerConfig;
use crate::errors::ServerError;
use crate::room_manager::RoomManager;
use crate::session::handle_connection;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info};

pub async fn run(config: ServerConfig) -> Result<(), ServerError> {
    let listener = TcpListener::bind(config.addr()).await?;
    let manager = Arc::new(Mutex::new(RoomManager::default()));
    info!(addr = %listener.local_addr()?, "play room server listening");

    loop {
        let (stream, addr) = listener.accept().await?;
        let manager = manager.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, manager).await {
                error!(%addr, ?err, "connection failed");
            }
        });
    }
}
