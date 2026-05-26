use crate::commands::{is_quit, parse_command};
use crate::config::ClientArgs;
use crate::errors::ClientError;
use crate::render::render_message;
use play_room_protocol::{decode_server, encode_client, ClientEnvelope, ClientRequest};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub async fn run_client(args: ClientArgs) -> Result<(), ClientError> {
    let stream = TcpStream::connect(args.addr()).await?;
    let (reader, mut writer) = stream.into_split();
    let mut server_lines = BufReader::new(reader).lines();

    let mut request_id = 1u64;
    let connect = ClientEnvelope {
        request_id,
        request: ClientRequest::Connect {
            name: args.name,
            reconnect_token: args.reconnect_token,
        },
    };
    writer
        .write_all(encode_client(&connect)?.as_bytes())
        .await?;
    request_id += 1;

    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = server_lines.next_line().await {
            match decode_server(&line) {
                Ok(message) => render_message(&message),
                Err(err) => eprintln!("protocol error: {err}"),
            }
        }
    });

    println!("type /help for commands");
    let mut stdin_lines = BufReader::new(io::stdin()).lines();
    while let Some(line) = stdin_lines.next_line().await? {
        if is_quit(&line) {
            break;
        }
        match parse_command(&line) {
            Ok(Some(request)) => {
                let envelope = ClientEnvelope {
                    request_id,
                    request,
                };
                request_id += 1;
                writer
                    .write_all(encode_client(&envelope)?.as_bytes())
                    .await?;
            }
            Ok(None) => {}
            Err(err) => eprintln!("{err}"),
        }
    }

    read_task.abort();
    Ok(())
}
