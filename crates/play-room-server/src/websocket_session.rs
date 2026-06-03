use crate::broadcast::channel;
use crate::errors::ServerError;
use crate::room_manager::RoomManager;
use crate::router::route;
use crate::scheduler::{now_ms, schedule_seat_expiry, schedule_spectator_expiry};
use futures_util::{SinkExt, StreamExt};
use play_room_protocol::{
    decode_client, encode_server, ClientRequest, ServerMessage, ServerResult,
};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, warn};

enum IncomingMessage {
    Text(String),
    Ignore,
    Close,
}

pub async fn handle_websocket_connection(
    stream: TcpStream,
    manager: Arc<Mutex<RoomManager>>,
) -> Result<(), ServerError> {
    let mut websocket = accept_async(stream).await?;
    let (tx, mut rx) = channel();

    let first = loop {
        let Some(message) = websocket.next().await else {
            return Ok(());
        };
        match incoming_message(message?) {
            IncomingMessage::Text(line) => break decode_client(&line)?,
            IncomingMessage::Ignore => {}
            IncomingMessage::Close => {
                websocket.flush().await?;
                return Ok(());
            }
        }
    };

    let ClientRequest::Connect {
        name,
        reconnect_token,
    } = first.request
    else {
        let msg = ServerMessage::Response {
            request_id: first.request_id,
            result: ServerResult::error("first request must be connect"),
        };
        websocket
            .send(Message::Text(encode_server(&msg)?.into()))
            .await?;
        return Ok(());
    };

    let connect_result = {
        let mut locked = manager.lock().await;
        match locked.try_connect_at(name, reconnect_token, tx, now_ms()) {
            Ok(connected) => {
                locked.welcome(&connected, first.request_id);
                locked.flush_messages(connected.messages.clone());
                Ok(connected)
            }
            Err(error) => Err(error.into_server_result()),
        }
    };
    let connected = match connect_result {
        Ok(connected) => connected,
        Err(result) => {
            let msg = ServerMessage::Response {
                request_id: first.request_id,
                result,
            };
            websocket
                .send(Message::Text(encode_server(&msg)?.into()))
                .await?;
            return Ok(());
        }
    };

    debug!(player_id = %connected.player_id, "websocket client connected");

    loop {
        tokio::select! {
            inbound = websocket.next() => {
                let Some(message) = inbound else {
                    break;
                };
                match incoming_message(message?) {
                    IncomingMessage::Text(line) => {
                        match decode_client(&line) {
                            Ok(envelope) => route(manager.clone(), connected.player_id.clone(), envelope).await,
                            Err(err) => {
                                let mut locked = manager.lock().await;
                                locked.respond(
                                    &connected.player_id,
                                    0,
                                    ServerResult::error(err.to_string()),
                                );
                            }
                        }
                    }
                    IncomingMessage::Ignore => {}
                    IncomingMessage::Close => {
                        websocket.flush().await?;
                        break;
                    }
                }
            }
            outbound = rx.recv() => {
                let Some(message) = outbound else {
                    break;
                };
                match encode_server(&message) {
                    Ok(line) => websocket.send(Message::Text(line.into())).await?,
                    Err(err) => warn!(?err, "failed to encode server message"),
                }
            }
        }
    }

    let outcome = {
        let mut locked = manager.lock().await;
        locked.disconnect(&connected.player_id, now_ms())
    };
    {
        let mut locked = manager.lock().await;
        locked.flush_messages(outcome.messages);
    }
    if let Some(expiry) = outcome.seat_expiry {
        schedule_seat_expiry(manager.clone(), expiry);
    }
    if let Some(expiry) = outcome.spectator_expiry {
        schedule_spectator_expiry(manager.clone(), expiry);
    }

    debug!(player_id = %connected.player_id, "websocket client disconnected");
    Ok(())
}

fn incoming_message(message: Message) -> IncomingMessage {
    match message {
        Message::Text(text) => IncomingMessage::Text(text.to_string()),
        Message::Binary(bytes) => match String::from_utf8(bytes.to_vec()) {
            Ok(text) => IncomingMessage::Text(text),
            Err(err) => {
                warn!(?err, "ignored non-utf8 websocket binary message");
                IncomingMessage::Ignore
            }
        },
        Message::Close(_) => IncomingMessage::Close,
        Message::Ping(_) | Message::Pong(_) => IncomingMessage::Ignore,
        Message::Frame(_) => IncomingMessage::Ignore,
    }
}
