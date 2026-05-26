use play_room_core::{PlayerId, RoomId, SessionToken};
use uuid::Uuid;

pub fn new_player_id() -> PlayerId {
    PlayerId::new(format!("player-{}", Uuid::new_v4()))
}

pub fn new_room_id() -> RoomId {
    RoomId::new(format!("room-{}", Uuid::new_v4()))
}

pub fn new_session_token() -> SessionToken {
    SessionToken::new(format!("session-{}", Uuid::new_v4()))
}
