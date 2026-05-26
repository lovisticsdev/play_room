mod broadcast;
mod config;
mod errors;
mod identity;
mod room_manager;
mod router;
mod scheduler;
mod server;
mod session;
mod websocket_session;

use clap::Parser;
use config::{ServerArgs, ServerConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), errors::ServerError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("play_room_server=info".parse().unwrap()),
        )
        .init();

    let args = ServerArgs::parse();
    let config = ServerConfig::load(args)?;
    server::run(config).await
}
