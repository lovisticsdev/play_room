use play_room_core::{GameRoom, PlayerRole, RoomPhase};

pub fn assert_lobby(room: &GameRoom) {
    assert!(matches!(room.phase(), RoomPhase::Lobby));
}

pub fn assert_finished(room: &GameRoom) {
    assert!(matches!(room.phase(), RoomPhase::Finished));
}

pub fn assert_participants(room: &GameRoom, expected: usize) {
    let actual = room
        .snapshot()
        .players
        .iter()
        .filter(|p| p.role == PlayerRole::Participant)
        .count();
    assert_eq!(actual, expected);
}
