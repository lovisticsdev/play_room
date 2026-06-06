use crate::errors::ClientError;
use play_room_core::{Move, RoomId, SUPPORTED_TARGET_SCORES};
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
            if !SUPPORTED_TARGET_SCORES.contains(&target_score) {
                return Err(ClientError::InvalidCommand(
                    "race target must be one of 1, 2, or 3".to_owned(),
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
    println!("  /race <1|2|3>");
    println!("  /again | /next");
    println!("  /ready | /unready");
    println!("  /move <rock|paper|scissors|lizard|spock>");
    println!("  /spectate [room_id|room_name] | /play");
    println!("  /ping");
    println!("  /quit");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ClientRequest {
        parse_command(input).unwrap().expect("expected request")
    }

    fn invalid(input: &str) -> String {
        parse_command(input).unwrap_err().to_string()
    }

    #[test]
    fn empty_input_and_help_do_not_emit_requests() {
        assert_eq!(parse_command("   ").unwrap(), None);
        assert_eq!(parse_command("/help").unwrap(), None);
    }

    #[test]
    fn parses_room_and_identity_commands() {
        assert_eq!(
            parse("/create Friday Finals"),
            ClientRequest::CreateRoom {
                name: "Friday Finals".to_owned(),
                rules: None,
            }
        );
        assert_eq!(
            parse("/join Friday Finals"),
            ClientRequest::JoinRoom {
                room_id: RoomId::new("Friday Finals"),
            }
        );
        assert_eq!(
            parse("/name Alice Prime"),
            ClientRequest::UpdateDisplayName {
                name: "Alice Prime".to_owned(),
            }
        );
    }

    #[test]
    fn parses_match_and_role_commands() {
        assert_eq!(
            parse("/race 3"),
            ClientRequest::UpdateMatchFormat { target_score: 3 }
        );
        assert_eq!(parse("/again"), ClientRequest::StartNextMatch);
        assert_eq!(parse("/next"), ClientRequest::StartNextMatch);
        assert_eq!(parse("/ready"), ClientRequest::SetReady { ready: true });
        assert_eq!(parse("/unready"), ClientRequest::SetReady { ready: false });
        assert_eq!(
            parse("/spectate"),
            ClientRequest::SetSpectator { spectator: true }
        );
        assert_eq!(
            parse("/spectate testroom"),
            ClientRequest::SpectateRoom {
                room_id: RoomId::new("testroom"),
            }
        );
        assert_eq!(
            parse("/play"),
            ClientRequest::SetSpectator { spectator: false }
        );
    }

    #[test]
    fn parses_moves_and_ping() {
        assert_eq!(
            parse("/move spock"),
            ClientRequest::SubmitMove { mv: Move::Spock }
        );
        assert_eq!(parse("/ping"), ClientRequest::Ping);
    }

    #[test]
    fn rejects_invalid_commands() {
        assert!(invalid("/create").contains("usage: /create"));
        assert!(invalid("/join").contains("usage: /join"));
        assert!(invalid("/name").contains("usage: /name"));
        assert!(invalid("/race").contains("usage: /race"));
        assert!(invalid("/race x").contains("positive number"));
        assert!(invalid("/race 5").contains("one of 1, 2, or 3"));
        assert!(invalid("/move").contains("usage: /move"));
        assert!(invalid("/move banana").contains("unknown move"));
        assert!(invalid("/wat").contains("unknown command"));
    }

    #[test]
    fn detects_quit_command_exactly_after_trim() {
        assert!(is_quit("  /quit "));
        assert!(!is_quit("/quit now"));
    }
}
