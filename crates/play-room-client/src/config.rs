use clap::Parser;
use play_room_core::SessionToken;

#[derive(Debug, Parser)]
#[command(name = "play-room-client", about = "Connect to a Play Room server")]
pub struct ClientArgs {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long, default_value_t = 7878)]
    pub port: u16,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub reconnect_token: Option<SessionToken>,
}

impl ClientArgs {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
