use crate::broadcast::{channel, send};
use crate::errors::ServerError;
use crate::room_manager::RoomManager;
use crate::router::route;
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
            result: ServerResult::Error {
                message: "first request must be connect".to_owned(),
            },
        };
        websocket
            .send(Message::Text(encode_server(&msg)?.into()))
            .await?;
        return Ok(());
    };

    let connected = {
        let mut locked = manager.lock().await;
        let connected = locked.connect(name, reconnect_token, tx.clone());
        locked.welcome(
            &connected.player_id,
            first.request_id,
            connected.reconnect_token.clone(),
        );
        connected
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
                            Err(err) => send(
                                &tx,
                                ServerMessage::Response {
                                    request_id: 0,
                                    result: ServerResult::Error {
                                        message: err.to_string(),
                                    },
                                },
                            ),
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

    let messages = {
        let mut locked = manager.lock().await;
        locked.disconnect(&connected.player_id)
    };
    {
        let locked = manager.lock().await;
        locked.flush_messages(messages);
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
