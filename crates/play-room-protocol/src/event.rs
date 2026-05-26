use play_room_core::{RoomEvent, RoomId, RoomSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ServerEvent {
    Notice { message: String },
    RoomEvent { room_id: RoomId, event: RoomEvent },
    RoomSnapshot { room: RoomSnapshot },
}
