use crate::room_manager::RoomManager;
use crate::scheduler::{now_ms, schedule_round_timeout};
use play_room_core::RoomCommand;
use play_room_protocol::{ClientEnvelope, ClientRequest, ServerResult};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn route(
    manager: Arc<Mutex<RoomManager>>,
    player_id: play_room_core::PlayerId,
    envelope: ClientEnvelope,
) {
    let request_id = envelope.request_id;
    match envelope.request {
        ClientRequest::Connect { .. } => {
            let locked = manager.lock().await;
            locked.respond(
                &player_id,
                request_id,
                ServerResult::Error {
                    message: "already connected".to_owned(),
                },
            );
        }
        ClientRequest::Ping => {
            let locked = manager.lock().await;
            locked.respond(&player_id, request_id, ServerResult::Pong);
        }
        ClientRequest::ListRooms => {
            let locked = manager.lock().await;
            locked.respond(
                &player_id,
                request_id,
                ServerResult::RoomList {
                    rooms: locked.list_rooms(),
                },
            );
        }
        ClientRequest::CreateRoom { name, rules } => {
            let mut locked = manager.lock().await;
            match locked.create_room(&player_id, name, rules) {
                Ok(room_id) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    let messages = locked.room_messages(&room_id, Vec::new());
                    locked.flush_messages(messages);
                }
                Err(message) => {
                    locked.respond(&player_id, request_id, ServerResult::Error { message })
                }
            }
        }
        ClientRequest::JoinRoom { room_id } => {
            let mut locked = manager.lock().await;
            match locked.join_room(&player_id, &room_id) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(message) => {
                    locked.respond(&player_id, request_id, ServerResult::Error { message })
                }
            }
        }
        ClientRequest::SpectateRoom { room_id } => {
            let mut locked = manager.lock().await;
            match locked.spectate_room(&player_id, &room_id) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(message) => {
                    locked.respond(&player_id, request_id, ServerResult::Error { message })
                }
            }
        }
        ClientRequest::LeaveRoom => {
            let mut locked = manager.lock().await;
            match locked.leave_current_room(&player_id) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(message) => {
                    locked.respond(&player_id, request_id, ServerResult::Error { message })
                }
            }
        }
        ClientRequest::SetReady { ready } => {
            let command = RoomCommand::SetReady {
                player_id: player_id.clone(),
                ready,
                now_ms: now_ms(),
            };
            apply_room_command(manager, player_id, request_id, command).await;
        }
        ClientRequest::SetSpectator { spectator } => {
            let command = RoomCommand::SetSpectator {
                player_id: player_id.clone(),
                spectator,
            };
            apply_room_command(manager, player_id, request_id, command).await;
        }
        ClientRequest::SubmitMove { mv } => {
            let command = RoomCommand::SubmitMove {
                player_id: player_id.clone(),
                mv,
                now_ms: now_ms(),
            };
            apply_room_command(manager, player_id, request_id, command).await;
        }
    }
}

async fn apply_room_command(
    manager: Arc<Mutex<RoomManager>>,
    player_id: play_room_core::PlayerId,
    request_id: u64,
    command: RoomCommand,
) {
    let mut locked = manager.lock().await;
    match locked.apply_to_current_room(&player_id, command) {
        Ok((room_id, events, timer)) => {
            locked.respond(&player_id, request_id, ServerResult::Ok);
            let messages = locked.room_messages(&room_id, events);
            locked.flush_messages(messages);
            drop(locked);
            if let Some((round, deadline_ms)) = timer {
                schedule_round_timeout(manager, room_id, round, deadline_ms);
            }
        }
        Err(message) => locked.respond(&player_id, request_id, ServerResult::Error { message }),
    }
}
