use crate::broadcast::{send, OutboundTx};
use play_room_core::{PlayerId, RoomEvent, RoomId, RoomSnapshot};
use play_room_protocol::{ServerEvent, ServerMessage};
use std::collections::BTreeMap;

pub type OutboundMessage = (PlayerId, ServerMessage);
pub type OutboundMessages = Vec<OutboundMessage>;

pub fn send_to(
    sessions: &BTreeMap<PlayerId, OutboundTx>,
    player_id: &PlayerId,
    message: ServerMessage,
) {
    if let Some(tx) = sessions.get(player_id) {
        send(tx, message);
    }
}

pub fn flush_messages(sessions: &BTreeMap<PlayerId, OutboundTx>, messages: OutboundMessages) {
    for (player_id, message) in messages {
        send_to(sessions, &player_id, message);
    }
}

pub fn room_messages(
    room_id: &RoomId,
    room_snapshot: RoomSnapshot,
    events: Vec<RoomEvent>,
    recipients: Vec<PlayerId>,
) -> OutboundMessages {
    let mut messages = Vec::new();
    for event in events {
        let msg = ServerMessage::Event {
            event: ServerEvent::RoomEvent {
                room_id: room_id.clone(),
                event,
            },
        };
        for recipient in &recipients {
            messages.push((recipient.clone(), msg.clone()));
        }
    }

    let snapshot = ServerMessage::Event {
        event: ServerEvent::RoomSnapshot {
            room: room_snapshot,
        },
    };
    for recipient in recipients {
        messages.push((recipient, snapshot.clone()));
    }
    messages
}
