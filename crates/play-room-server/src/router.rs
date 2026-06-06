use crate::room_manager::{RoomManager, RoomManagerError};
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
            let mut locked = manager.lock().await;
            locked.respond(
                &player_id,
                request_id,
                ServerResult::error("already connected"),
            );
        }
        ClientRequest::Ping => {
            let mut locked = manager.lock().await;
            locked.respond(&player_id, request_id, ServerResult::Pong);
        }
        ClientRequest::ListRooms => {
            let mut locked = manager.lock().await;
            let rooms = locked.list_rooms();
            locked.respond(&player_id, request_id, ServerResult::RoomList { rooms });
        }
        ClientRequest::CreateRoom { name, rules } => {
            let mut locked = manager.lock().await;
            match locked.create_room(&player_id, name, rules) {
                Ok((_room_id, messages)) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(error) => respond_error(&mut locked, &player_id, request_id, error),
            }
        }
        ClientRequest::JoinRoom { room_id } => {
            let mut locked = manager.lock().await;
            match locked.join_room(&player_id, &room_id) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(error) => respond_error(&mut locked, &player_id, request_id, error),
            }
        }
        ClientRequest::SpectateRoom { room_id } => {
            let mut locked = manager.lock().await;
            match locked.spectate_room(&player_id, &room_id) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(error) => respond_error(&mut locked, &player_id, request_id, error),
            }
        }
        ClientRequest::EnterRoom { room_id, mode } => {
            let mut locked = manager.lock().await;
            match locked.enter_room(&player_id, &room_id, mode) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(error) => respond_error(&mut locked, &player_id, request_id, error),
            }
        }
        ClientRequest::UpdateDisplayName { name } => {
            let mut locked = manager.lock().await;
            match locked.update_display_name(&player_id, name) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(error) => respond_error(&mut locked, &player_id, request_id, error),
            }
        }
        ClientRequest::UpdateMatchFormat { target_score } => {
            let command = RoomCommand::UpdateMatchFormat {
                player_id: player_id.clone(),
                target_score,
            };
            apply_room_command(manager, player_id, request_id, command).await;
        }
        ClientRequest::LeaveRoom => {
            let mut locked = manager.lock().await;
            match locked.leave_current_room(&player_id) {
                Ok(messages) => {
                    locked.respond(&player_id, request_id, ServerResult::Ok);
                    locked.flush_messages(messages);
                }
                Err(error) => respond_error(&mut locked, &player_id, request_id, error),
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
        ClientRequest::StartNextMatch => {
            let command = RoomCommand::StartNextMatch {
                player_id: player_id.clone(),
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
        Err(error) => respond_error(&mut locked, &player_id, request_id, error),
    }
}

fn respond_error(
    manager: &mut RoomManager,
    player_id: &play_room_core::PlayerId,
    request_id: u64,
    error: RoomManagerError,
) {
    manager.respond(player_id, request_id, error.into_server_result());
}
