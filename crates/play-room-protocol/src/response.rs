use play_room_core::{PlayerId, RoomSnapshot, RoomSummary, SessionToken};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum ServerResult {
    Ok,
    Error {
        message: String,
    },
    Welcome {
        player_id: PlayerId,
        reconnect_token: SessionToken,
        protocol_version: u16,
    },
    RoomList {
        rooms: Vec<RoomSummary>,
    },
    RoomSnapshot {
        room: RoomSnapshot,
    },
    Pong,
}
