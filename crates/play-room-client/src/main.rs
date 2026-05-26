mod commands;
mod config;
mod connection;
mod errors;
mod render;

use clap::Parser;
use config::ClientArgs;

#[tokio::main]
async fn main() -> Result<(), errors::ClientError> {
    let args = ClientArgs::parse();
    connection::run_client(args).await
}
