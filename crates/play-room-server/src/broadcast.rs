use play_room_protocol::ServerMessage;
use tokio::sync::mpsc;

pub const DEFAULT_OUTBOUND_QUEUE_CAPACITY: usize = 256;

pub type OutboundTx = mpsc::Sender<ServerMessage>;
pub type OutboundRx = mpsc::Receiver<ServerMessage>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendFailure {
    Full,
    Closed,
}

pub fn channel() -> (OutboundTx, OutboundRx) {
    channel_with_capacity(DEFAULT_OUTBOUND_QUEUE_CAPACITY)
}

pub fn channel_with_capacity(capacity: usize) -> (OutboundTx, OutboundRx) {
    mpsc::channel(capacity.max(1))
}

pub fn send(tx: &OutboundTx, message: ServerMessage) -> Result<(), SendFailure> {
    match tx.try_send(message) {
        Ok(()) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => Err(SendFailure::Full),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(SendFailure::Closed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use play_room_protocol::ServerResult;

    fn response_message() -> ServerMessage {
        ServerMessage::Response {
            request_id: 1,
            result: ServerResult::Ok,
        }
    }

    #[test]
    fn bounded_channel_reports_full_without_waiting() {
        let (tx, _rx) = channel_with_capacity(1);

        assert_eq!(send(&tx, response_message()), Ok(()));
        assert_eq!(send(&tx, response_message()), Err(SendFailure::Full));
    }

    #[test]
    fn closed_channel_reports_closed() {
        let (tx, rx) = channel_with_capacity(1);
        drop(rx);

        assert_eq!(send(&tx, response_message()), Err(SendFailure::Closed));
    }
}
