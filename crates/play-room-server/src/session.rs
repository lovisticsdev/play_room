use crate::broadcast::{channel, send};
use crate::errors::ServerError;
use crate::room_manager::RoomManager;
use crate::router::route;
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
            result: ServerResult::Error {
                message: "first request must be connect".to_owned(),
            },
        };
        writer.write_all(encode_server(&msg)?.as_bytes()).await?;
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

    let writer_task = tokio::spawn(async move {
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

    while let Some(line) = lines.next_line().await? {
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

    let messages = {
        let mut locked = manager.lock().await;
        locked.disconnect(&connected.player_id)
    };
    {
        let locked = manager.lock().await;
        locked.flush_messages(messages);
    }

    writer_task.abort();
    debug!(player_id = %connected.player_id, "client disconnected");
    Ok(())
}
