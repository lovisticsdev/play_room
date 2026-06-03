use crate::errors::ServerError;
use crate::room_manager::RoomManagerLimits;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "play-room-server", about = "Run the Play Room server")]
pub struct ServerArgs {
    #[arg(long, default_value = "examples/server.toml")]
    pub config: PathBuf,

    #[arg(long)]
    pub host: Option<String>,

    #[arg(long)]
    pub port: Option<u16>,
}

const DEFAULT_ABANDONED_SESSION_TTL_SECONDS: u64 = 30 * 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_rooms: usize,
    pub max_clients: usize,
    #[serde(default = "default_abandoned_session_ttl_seconds")]
    pub abandoned_session_ttl_seconds: u64,
}

fn default_abandoned_session_ttl_seconds() -> u64 {
    DEFAULT_ABANDONED_SESSION_TTL_SECONDS
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 7878,
            max_rooms: 128,
            max_clients: 512,
            abandoned_session_ttl_seconds: DEFAULT_ABANDONED_SESSION_TTL_SECONDS,
        }
    }
}

impl ServerConfig {
    pub fn load(args: ServerArgs) -> Result<Self, ServerError> {
        let mut cfg = if args.config.exists() {
            let text = fs::read_to_string(&args.config)?;
            toml::from_str::<ServerConfig>(&text).map_err(|e| ServerError::Config(e.to_string()))?
        } else {
            ServerConfig::default()
        };

        if let Some(host) = args.host {
            cfg.host = host;
        }
        if let Some(port) = args.port {
            cfg.port = port;
        }
        Ok(cfg)
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn room_manager_limits(&self) -> RoomManagerLimits {
        RoomManagerLimits {
            max_rooms: self.max_rooms,
            max_clients: self.max_clients,
            abandoned_session_ttl_ms: self.abandoned_session_ttl_seconds.saturating_mul(1_000),
        }
    }
}
