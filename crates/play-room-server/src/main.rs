mod broadcast;
mod config;
mod errors;
mod expiry;
mod fanout;
mod identity;
mod membership;
mod room_lifecycle;
mod room_manager;
mod room_registry;
mod router;
mod scheduler;
mod server;
mod session;
mod session_registry;
mod websocket_session;

use clap::Parser;
use config::{ServerArgs, ServerConfig};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), errors::ServerError> {
    let default_directive = "play_room_server=info"
        .parse::<Directive>()
        .map_err(|err| errors::ServerError::Config(format!("invalid log directive: {err}")))?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(default_directive))
        .init();

    let args = ServerArgs::parse();
    let config = ServerConfig::load(args)?;
    server::run(config).await
}
