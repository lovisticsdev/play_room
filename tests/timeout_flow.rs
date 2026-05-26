mod common;

use common::{id, ready, two_player_room};
use play_room_core::{Move, RoomCommand, RoomPhase};

#[test]
fn timeout_gives_point_to_player_who_moved() {
    let mut room = two_player_room();
    ready(&mut room, "alice", 1000);
    ready(&mut room, "bob", 1000);

    let (round, deadline_ms) = match room.phase() {
        RoomPhase::InRound { round, deadline_ms } => (*round, *deadline_ms),
        _ => panic!("round should be active"),
    };

    room.apply(RoomCommand::SubmitMove {
        player_id: id("alice"),
        mv: Move::Paper,
        now_ms: 1001,
    })
    .unwrap();
    room.apply(RoomCommand::TimeoutRound {
        round,
        now_ms: deadline_ms,
    })
    .unwrap();
    assert_eq!(room.snapshot().scoreboard[0].player_id, id("alice"));
}
