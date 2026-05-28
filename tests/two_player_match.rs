mod common;

use common::{id, ready, two_player_room};
use play_room_core::{Move, RoomCommand, RoomPhase};

#[test]
fn two_players_can_finish_a_best_of_three_match() {
    let mut room = two_player_room();

    for now in [1000, 2000] {
        ready(&mut room, "alice", now);
        ready(&mut room, "bob", now);
        room.apply(RoomCommand::SubmitMove {
            player_id: id("alice"),
            mv: Move::Rock,
            now_ms: now + 1,
        })
        .unwrap();
        room.apply(RoomCommand::SubmitMove {
            player_id: id("bob"),
            mv: Move::Scissors,
            now_ms: now + 2,
        })
        .unwrap();
    }

    assert!(matches!(
        room.phase(),
        RoomPhase::Finished {
            winner: Some(winner)
        } if winner == &id("alice")
    ));
    assert_eq!(room.snapshot().scoreboard[0].player_id, id("alice"));
    assert_eq!(room.snapshot().scoreboard[0].score, 2);
}
