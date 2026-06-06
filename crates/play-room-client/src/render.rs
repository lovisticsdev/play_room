use play_room_core::{RoomEvent, RoomPhase};
use play_room_protocol::{ServerEvent, ServerMessage, ServerResult};

pub fn render_message(message: &ServerMessage) {
    match message {
        ServerMessage::Response { request_id, result } => render_response(*request_id, result),
        ServerMessage::Event { event } => render_event(event),
    }
}

fn render_response(request_id: u64, result: &ServerResult) {
    match result {
        ServerResult::Ok => println!("[ok #{request_id}]"),
        ServerResult::Error {
            message,
            suggestions,
            ..
        } => {
            println!("[error #{request_id}] {message}");
            if !suggestions.is_empty() {
                println!("suggestions: {}", suggestions.join(", "));
            }
        }
        ServerResult::Welcome {
            player_id,
            display_name,
            reconnect_token,
            protocol_version,
            reconnected,
            stale_token_replaced,
            room_restored,
        } => {
            println!(
                "connected as {display_name} ({player_id}) using protocol v{protocol_version}"
            );
            println!("reconnect token: {reconnect_token}");
            if *stale_token_replaced {
                println!("reconnect status: stale token replaced with a fresh session");
            } else if *reconnected {
                let room_status = if *room_restored {
                    "room restored"
                } else {
                    "no room restored"
                };
                println!("reconnect status: identity restored, {room_status}");
            }
        }
        ServerResult::RoomList { rooms } => {
            if rooms.is_empty() {
                println!("no rooms");
            } else {
                for room in rooms {
                    println!(
                        "{} | {} | players {}/{} | spectators {} | race to {}",
                        room.id,
                        room.name,
                        room.players,
                        room.max_players,
                        room.spectators,
                        room.target_score
                    );
                }
            }
        }
        ServerResult::RoomSnapshot { room } => {
            println!("room {} ({})", room.name, room.id);
        }
        ServerResult::Pong => println!("pong"),
    }
}

fn render_event(event: &ServerEvent) {
    match event {
        ServerEvent::Notice { message } => println!("[notice] {message}"),
        ServerEvent::RoomEvent { room_id, event } => match event {
            RoomEvent::PlayerJoined { name, role, .. } => {
                println!("[{room_id}] {name} joined as {role:?}")
            }
            RoomEvent::PlayerLeft { player_id } => println!("[{room_id}] {player_id} left"),
            RoomEvent::PlayerDisconnected { player_id } => {
                println!("[{room_id}] {player_id} disconnected")
            }
            RoomEvent::PlayerReconnected { player_id } => {
                println!("[{room_id}] {player_id} reconnected")
            }
            RoomEvent::PlayerRenamed { player_id, name } => {
                println!("[{room_id}] {player_id} renamed to {name}")
            }
            RoomEvent::MatchFormatChanged { target_score } => {
                println!("[{room_id}] match format changed: race to {target_score}")
            }
            RoomEvent::ReadyChanged { player_id, ready } => {
                println!("[{room_id}] {player_id} ready={ready}")
            }
            RoomEvent::RoleChanged { player_id, role } => {
                println!("[{room_id}] {player_id} is now {role:?}")
            }
            RoomEvent::RoundStarted { round, deadline_ms } => {
                println!("[{room_id}] round {round} started; deadline_ms={deadline_ms}")
            }
            RoomEvent::MoveAccepted { player_id } => {
                println!("[{room_id}] move accepted from {player_id}")
            }
            RoomEvent::RoundResolved { result } => println!(
                "[{room_id}] round {} resolved: {:?}",
                result.round, result.outcome
            ),
            RoomEvent::GameEnded { winner } => {
                println!("[{room_id}] game ended; winner={winner:?}")
            }
            RoomEvent::MatchReset { requested_by } => {
                println!("[{room_id}] next match started by {requested_by}")
            }
            RoomEvent::HostChanged { host_id } => println!("[{room_id}] host changed: {host_id:?}"),
        },
        ServerEvent::RoomSnapshot { room } => {
            let phase = match &room.phase {
                RoomPhase::Lobby => "lobby".to_owned(),
                RoomPhase::InRound { round, .. } => format!("round {round}"),
                RoomPhase::Finished { winner } => format!("finished winner={winner:?}"),
            };
            println!("[snapshot] {} | {} | {}", room.id, room.name, phase);
            for player in &room.players {
                println!(
                    "  - {} {} role={:?} ready={} score={} connected={}",
                    player.id,
                    player.name,
                    player.role,
                    player.ready,
                    player.score,
                    player.connected
                );
            }
        }
    }
}
