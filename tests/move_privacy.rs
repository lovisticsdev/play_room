mod common;

use common::{id, ready, two_player_room};
use play_room_core::{Move, RoomCommand, RoomEvent};

#[test]
fn move_accepted_event_hides_selected_move_until_round_resolves() {
    let mut room = two_player_room();
    ready(&mut room, "alice", 1000);
    ready(&mut room, "bob", 1000);

    let first_events = room
        .apply(RoomCommand::SubmitMove {
            player_id: id("alice"),
            mv: Move::Rock,
            now_ms: 1001,
        })
        .unwrap();
    let accepted = first_events
        .iter()
        .find(|event| matches!(event, RoomEvent::MoveAccepted { .. }))
        .expect("expected a move-accepted event");

    assert!(matches!(
        accepted,
        RoomEvent::MoveAccepted { player_id } if player_id == &id("alice")
    ));
    let accepted_json = serde_json::to_value(accepted).unwrap();
    assert_eq!(accepted_json["event"], "move_accepted");
    assert!(accepted_json.get("mv").is_none());
    assert!(!accepted_json.to_string().contains("rock"));

    let second_events = room
        .apply(RoomCommand::SubmitMove {
            player_id: id("bob"),
            mv: Move::Scissors,
            now_ms: 1002,
        })
        .unwrap();
    let result = second_events
        .iter()
        .find_map(|event| match event {
            RoomEvent::RoundResolved { result } => Some(result),
            _ => None,
        })
        .expect("expected a resolved round after both moves");

    assert_eq!(result.submitted.get(&id("alice")), Some(&Some(Move::Rock)));
    assert_eq!(
        result.submitted.get(&id("bob")),
        Some(&Some(Move::Scissors))
    );
    let resolved_json = serde_json::to_value(result).unwrap().to_string();
    assert!(resolved_json.contains("rock"));
    assert!(resolved_json.contains("scissors"));
}
