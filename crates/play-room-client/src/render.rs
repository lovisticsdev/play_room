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
        ServerResult::Error { message } => println!("[error #{request_id}] {message}"),
        ServerResult::Welcome {
            player_id,
            reconnect_token,
            protocol_version,
        } => {
            println!("connected as {player_id} using protocol v{protocol_version}");
            println!("reconnect token: {reconnect_token}");
        }
        ServerResult::RoomList { rooms } => {
            if rooms.is_empty() {
                println!("no rooms");
            } else {
                for room in rooms {
                    println!(
                        "{} | {} | players {}/{} | spectators {}",
                        room.id, room.name, room.players, room.max_players, room.spectators
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
            RoomEvent::ReadyChanged { player_id, ready } => {
                println!("[{room_id}] {player_id} ready={ready}")
            }
            RoomEvent::RoleChanged { player_id, role } => {
                println!("[{room_id}] {player_id} is now {role:?}")
            }
            RoomEvent::RoundStarted { round, deadline_ms } => {
                println!("[{room_id}] round {round} started; deadline_ms={deadline_ms}")
            }
            RoomEvent::MoveAccepted { player_id, .. } => {
                println!("[{room_id}] move accepted from {player_id}")
            }
            RoomEvent::RoundResolved { result } => println!(
                "[{room_id}] round {} resolved: {:?}",
                result.round, result.outcome
            ),
            RoomEvent::GameEnded { winner } => {
                println!("[{room_id}] game ended; winner={winner:?}")
            }
            RoomEvent::HostChanged { host_id } => println!("[{room_id}] host changed: {host_id:?}"),
        },
        ServerEvent::RoomSnapshot { room } => {
            let phase = match &room.phase {
                RoomPhase::Lobby => "lobby".to_owned(),
                RoomPhase::InRound { round, .. } => format!("round {round}"),
                RoomPhase::Finished => "finished".to_owned(),
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
