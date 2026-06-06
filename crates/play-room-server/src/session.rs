use crate::broadcast::channel;
use crate::errors::ServerError;
use crate::room_manager::RoomManager;
use crate::router::route;
use crate::scheduler::{
    flush_and_schedule, now_ms, schedule_seat_expiry, schedule_spectator_expiry,
};
use play_room_protocol::{
    decode_client, encode_server, ClientRequest, ServerMessage, ServerResult,
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub async fn handle_connection(
    stream: TcpStream,
    manager: Arc<Mutex<RoomManager>>,
) -> Result<(), ServerError> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let (tx, mut rx) = channel();

    let Some(line) = lines.next_line().await? else {
        return Ok(());
    };
    let first = decode_client(&line)?;
    let ClientRequest::Connect {
        name,
        reconnect_token,
    } = first.request
    else {
        let msg = ServerMessage::Response {
            request_id: first.request_id,
            result: ServerResult::error("first request must be connect"),
        };
        writer.write_all(encode_server(&msg)?.as_bytes()).await?;
        return Ok(());
    };

    let connect_result = {
        let mut locked = manager.lock().await;
        match locked.try_connect_at(name, reconnect_token, tx, now_ms()) {
            Ok(connected) => {
                let mut messages = locked.welcome_messages(&connected, first.request_id);
                messages.extend(connected.messages.clone());
                Ok((connected, messages))
            }
            Err(error) => Err(error.into_server_result()),
        }
    };
    let (connected, connect_messages) = match connect_result {
        Ok(result) => result,
        Err(result) => {
            let msg = ServerMessage::Response {
                request_id: first.request_id,
                result,
            };
            writer.write_all(encode_server(&msg)?.as_bytes()).await?;
            return Ok(());
        }
    };
    flush_and_schedule(manager.clone(), connect_messages).await;

    let mut writer_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match encode_server(&message) {
                Ok(line) => {
                    if writer.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Err(err) => warn!(?err, "failed to encode server message"),
            }
        }
    });

    debug!(player_id = %connected.player_id, "client connected");

    let mut writer_finished = false;
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line? else {
                    break;
                };
                match decode_client(&line) {
                    Ok(envelope) => route(manager.clone(), connected.player_id.clone(), envelope).await,
                    Err(err) => {
                        let messages = RoomManager::response_messages(
                            &connected.player_id,
                            0,
                            ServerResult::error(err.to_string()),
                        );
                        flush_and_schedule(manager.clone(), messages).await;
                    }
                }
            }
            result = &mut writer_task => {
                writer_finished = true;
                if let Err(err) = result {
                    warn!(?err, "connection writer task failed");
                }
                break;
            }
        }
    }

    let outcome = {
        let mut locked = manager.lock().await;
        locked.disconnect(&connected.player_id, connected.connection_id, now_ms())
    };
    flush_and_schedule(manager.clone(), outcome.messages).await;
    if let Some(expiry) = outcome.seat_expiry {
        schedule_seat_expiry(manager.clone(), expiry);
    }
    if let Some(expiry) = outcome.spectator_expiry {
        schedule_spectator_expiry(manager.clone(), expiry);
    }

    if !writer_finished {
        writer_task.abort();
    }
    debug!(player_id = %connected.player_id, "client disconnected");
    Ok(())
}
