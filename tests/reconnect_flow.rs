mod common;

use common::{id, room_with_host};
use play_room_core::RoomCommand;

#[test]
fn player_can_disconnect_and_reconnect() {
    let mut room = room_with_host("room", "alice");

    room.apply(RoomCommand::Disconnect {
        player_id: id("alice"),
    })
    .unwrap();
    assert!(!room.snapshot().players[0].connected);

    room.apply(RoomCommand::Reconnect {
        player_id: id("alice"),
    })
    .unwrap();
    assert!(room.snapshot().players[0].connected);
}
