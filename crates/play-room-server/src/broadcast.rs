use play_room_protocol::ServerMessage;
use tokio::sync::mpsc;

pub type OutboundTx = mpsc::UnboundedSender<ServerMessage>;
pub type OutboundRx = mpsc::UnboundedReceiver<ServerMessage>;

pub fn channel() -> (OutboundTx, OutboundRx) {
    mpsc::unbounded_channel()
}

pub fn send(tx: &OutboundTx, message: ServerMessage) {
    let _ = tx.send(message);
}
