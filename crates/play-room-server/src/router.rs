use crate::fanout::OutboundMessages;
use crate::room_manager::{RoomManager, RoomManagerError};
use crate::scheduler::{flush_and_schedule, now_ms, schedule_round_timeout};
use play_room_core::{PlayerId, RoomCommand};
use play_room_protocol::{ClientEnvelope, ClientRequest, ServerResult};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn route(
    manager: Arc<Mutex<RoomManager>>,
    player_id: PlayerId,
    envelope: ClientEnvelope,
) {
    let request_id = envelope.request_id;
    match envelope.request {
        ClientRequest::Connect { .. } => {
            respond(
                manager,
                player_id,
                request_id,
                ServerResult::error("already connected"),
            )
            .await;
        }
        ClientRequest::Ping => {
            respond(manager, player_id, request_id, ServerResult::Pong).await;
        }
        ClientRequest::ListRooms => {
            let rooms = {
                let locked = manager.lock().await;
                locked.list_rooms()
            };
            respond(
                manager,
                player_id,
                request_id,
                ServerResult::RoomList { rooms },
            )
            .await;
        }
        ClientRequest::CreateRoom { name, rules } => {
            let result = {
                let mut locked = manager.lock().await;
                locked.create_room(&player_id, name, rules)
            };
            match result {
                Ok((_room_id, messages)) => {
                    respond_with_messages(manager, player_id, request_id, messages).await;
                }
                Err(error) => respond_error(manager, player_id, request_id, error).await,
            }
        }
        ClientRequest::JoinRoom { room_id } => {
            let result = {
                let mut locked = manager.lock().await;
                locked.join_room(&player_id, &room_id)
            };
            match result {
                Ok(messages) => {
                    respond_with_messages(manager, player_id, request_id, messages).await
                }
                Err(error) => respond_error(manager, player_id, request_id, error).await,
            }
        }
        ClientRequest::SpectateRoom { room_id } => {
            let result = {
                let mut locked = manager.lock().await;
                locked.spectate_room(&player_id, &room_id)
            };
            match result {
                Ok(messages) => {
                    respond_with_messages(manager, player_id, request_id, messages).await
                }
                Err(error) => respond_error(manager, player_id, request_id, error).await,
            }
        }
        ClientRequest::EnterRoom { room_id, mode } => {
            let result = {
                let mut locked = manager.lock().await;
                locked.enter_room(&player_id, &room_id, mode)
            };
            match result {
                Ok(messages) => {
                    respond_with_messages(manager, player_id, request_id, messages).await
                }
                Err(error) => respond_error(manager, player_id, request_id, error).await,
            }
        }
        ClientRequest::UpdateDisplayName { name } => {
            let result = {
                let mut locked = manager.lock().await;
                locked.update_display_name(&player_id, name)
            };
            match result {
                Ok(messages) => {
                    respond_with_messages(manager, player_id, request_id, messages).await
                }
                Err(error) => respond_error(manager, player_id, request_id, error).await,
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
            let result = {
                let mut locked = manager.lock().await;
                locked.leave_current_room(&player_id)
            };
            match result {
                Ok(messages) => {
                    respond_with_messages(manager, player_id, request_id, messages).await
                }
                Err(error) => respond_error(manager, player_id, request_id, error).await,
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
    player_id: PlayerId,
    request_id: u64,
    command: RoomCommand,
) {
    let result = {
        let mut locked = manager.lock().await;
        locked
            .apply_to_current_room(&player_id, command)
            .map(|(room_id, events, timer)| {
                let messages = locked.room_messages(&room_id, events);
                (room_id, messages, timer)
            })
    };

    match result {
        Ok((room_id, messages, timer)) => {
            respond_with_messages(manager.clone(), player_id, request_id, messages).await;
            if let Some((round, deadline_ms)) = timer {
                schedule_round_timeout(manager, room_id, round, deadline_ms);
            }
        }
        Err(error) => respond_error(manager, player_id, request_id, error).await,
    }
}

async fn respond_with_messages(
    manager: Arc<Mutex<RoomManager>>,
    player_id: PlayerId,
    request_id: u64,
    mut messages: OutboundMessages,
) {
    let mut response = RoomManager::response_messages(&player_id, request_id, ServerResult::Ok);
    response.append(&mut messages);
    flush_and_schedule(manager, response).await;
}

async fn respond(
    manager: Arc<Mutex<RoomManager>>,
    player_id: PlayerId,
    request_id: u64,
    result: ServerResult,
) {
    flush_and_schedule(
        manager,
        RoomManager::response_messages(&player_id, request_id, result),
    )
    .await;
}

async fn respond_error(
    manager: Arc<Mutex<RoomManager>>,
    player_id: PlayerId,
    request_id: u64,
    error: RoomManagerError,
) {
    respond(manager, player_id, request_id, error.into_server_result()).await;
}
