use play_room_core::{Player, PlayerId, RoomId};
use rand::{distributions::Alphanumeric, Rng};

pub fn player(name: &str) -> Player {
    Player::participant(PlayerId::new(format!("player-{name}")), name)
}

pub fn room_id() -> RoomId {
    RoomId::new(format!("room-{}", random_suffix()))
}

pub fn random_suffix() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}
