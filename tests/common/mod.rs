#![allow(dead_code)]

use play_room_core::{GameRoom, Player, PlayerId, RoomCommand, RoomId};

pub fn id(value: &str) -> PlayerId {
    PlayerId::new(value)
}

pub fn participant(name: &str) -> Player {
    Player::participant(id(name), name)
}

pub fn spectator(name: &str) -> Player {
    Player::spectator(id(name), name)
}

pub fn room_with_host(room_id: &str, host_name: &str) -> GameRoom {
    GameRoom::new(
        RoomId::new(room_id),
        room_id,
        Default::default(),
        participant(host_name),
    )
    .unwrap()
}

pub fn two_player_room() -> GameRoom {
    let mut room = room_with_host("room", "alice");
    room.apply(RoomCommand::Join {
        player: participant("bob"),
    })
    .unwrap();
    room
}

pub fn ready(room: &mut GameRoom, player: &str, now_ms: u64) {
    room.apply(RoomCommand::SetReady {
        player_id: id(player),
        ready: true,
        now_ms,
    })
    .unwrap();
}
