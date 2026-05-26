mod common;

use common::{id, participant, ready, room_with_host, spectator};
use play_room_core::{Move, PlayerRole, RoomCommand};

#[test]
fn spectators_cannot_submit_moves() {
    let mut room = room_with_host("room", "alice");
    room.apply(RoomCommand::Join {
        player: participant("bob"),
    })
    .unwrap();
    room.apply(RoomCommand::Join {
        player: spectator("eve"),
    })
    .unwrap();

    let eve_view = room
        .snapshot()
        .players
        .into_iter()
        .find(|p| p.id == id("eve"))
        .unwrap();
    assert_eq!(eve_view.role, PlayerRole::Spectator);

    ready(&mut room, "alice", 0);
    ready(&mut room, "bob", 0);
    let err = room
        .apply(RoomCommand::SubmitMove {
            player_id: id("eve"),
            mv: Move::Rock,
            now_ms: 1,
        })
        .unwrap_err();
    assert!(err.to_string().contains("spectator"));
}
