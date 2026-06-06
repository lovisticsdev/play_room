use crate::errors::ClientError;
use play_room_core::{Move, RoomId};
use play_room_protocol::ClientRequest;

pub fn parse_command(input: &str) -> Result<Option<ClientRequest>, ClientError> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    let mut parts = input.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(None);
    };

    match command {
        "/help" => {
            print_help();
            Ok(None)
        }
        "/rooms" => Ok(Some(ClientRequest::ListRooms)),
        "/create" => {
            let name = parts.collect::<Vec<_>>().join(" ");
            if name.is_empty() {
                return Err(ClientError::InvalidCommand(
                    "usage: /create <room name>".to_owned(),
                ));
            }
            Ok(Some(ClientRequest::CreateRoom { name, rules: None }))
        }
        "/join" => {
            let room = parts.collect::<Vec<_>>().join(" ");
            if room.is_empty() {
                return Err(ClientError::InvalidCommand(
                    "usage: /join <room_id|room_name>".to_owned(),
                ));
            };
            Ok(Some(ClientRequest::JoinRoom {
                room_id: RoomId::new(room),
            }))
        }
        "/leave" => Ok(Some(ClientRequest::LeaveRoom)),
        "/name" => {
            let name = parts.collect::<Vec<_>>().join(" ");
            if name.is_empty() {
                return Err(ClientError::InvalidCommand(
                    "usage: /name <display name>".to_owned(),
                ));
            }
            Ok(Some(ClientRequest::UpdateDisplayName { name }))
        }
        "/race" => {
            let Some(value) = parts.next() else {
                return Err(ClientError::InvalidCommand(
                    "usage: /race <points>".to_owned(),
                ));
            };
            let target_score = value.parse::<u32>().map_err(|_| {
                ClientError::InvalidCommand("race target must be a positive number".to_owned())
            })?;
            if target_score == 0 {
                return Err(ClientError::InvalidCommand(
                    "race target must be at least 1".to_owned(),
                ));
            }
            Ok(Some(ClientRequest::UpdateMatchFormat { target_score }))
        }
        "/again" | "/next" => Ok(Some(ClientRequest::StartNextMatch)),
        "/ready" => Ok(Some(ClientRequest::SetReady { ready: true })),
        "/unready" => Ok(Some(ClientRequest::SetReady { ready: false })),
        "/spectate" => {
            let room = parts.collect::<Vec<_>>().join(" ");
            if room.is_empty() {
                Ok(Some(ClientRequest::SetSpectator { spectator: true }))
            } else {
                Ok(Some(ClientRequest::SpectateRoom {
                    room_id: RoomId::new(room),
                }))
            }
        }
        "/play" => Ok(Some(ClientRequest::SetSpectator { spectator: false })),
        "/move" => {
            let Some(value) = parts.next() else {
                return Err(ClientError::InvalidCommand(
                    "usage: /move <rock|paper|scissors|lizard|spock>".to_owned(),
                ));
            };
            let Some(mv) = Move::parse(value) else {
                return Err(ClientError::InvalidCommand(format!(
                    "unknown move: {value}"
                )));
            };
            Ok(Some(ClientRequest::SubmitMove { mv }))
        }
        "/ping" => Ok(Some(ClientRequest::Ping)),
        "/quit" => Ok(None),
        other => Err(ClientError::InvalidCommand(format!(
            "unknown command: {other}; use /help"
        ))),
    }
}

pub fn is_quit(input: &str) -> bool {
    input.trim() == "/quit"
}

fn print_help() {
    println!("commands:");
    println!("  /rooms");
    println!("  /create <room name>");
    println!("  /join <room_id|room_name>");
    println!("  /leave");
    println!("  /name <display name>");
    println!("  /race <points>");
    println!("  /again | /next");
    println!("  /ready | /unready");
    println!("  /move <rock|paper|scissors|lizard|spock>");
    println!("  /spectate [room_id|room_name] | /play");
    println!("  /ping");
    println!("  /quit");
}
