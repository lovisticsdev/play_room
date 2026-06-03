use crate::identity::new_room_id;
use play_room_core::{
    CoreError, GameRoom, GameRules, Player, PlayerId, PlayerRole, RoomEvent, RoomId,
};

pub type RoundTimer = (u32, u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerNameConflict {
    pub name: String,
    pub connected: bool,
}

pub fn create_room(
    owner_id: &PlayerId,
    room_name: &str,
    rules: GameRules,
    player_name: String,
) -> Result<(RoomId, GameRoom), CoreError> {
    let room_id = new_room_id();
    let host = Player::participant(owner_id.clone(), player_name);
    let room = GameRoom::new(room_id.clone(), room_name.to_owned(), rules, host)?;
    Ok((room_id, room))
}

pub fn player_for_role(player_id: &PlayerId, player_name: String, role: PlayerRole) -> Player {
    match role {
        PlayerRole::Participant => Player::participant(player_id.clone(), player_name),
        PlayerRole::Spectator => Player::spectator(player_id.clone(), player_name),
    }
}

pub fn player_name_conflict(
    room: &GameRoom,
    player_id: &PlayerId,
    player_name: &str,
) -> Option<PlayerNameConflict> {
    let existing = room.player_named(player_name)?;
    (existing.id != *player_id).then_some(PlayerNameConflict {
        name: existing.name,
        connected: existing.connected,
    })
}

pub fn round_timer(events: &[RoomEvent]) -> Option<RoundTimer> {
    events.iter().find_map(|event| {
        if let RoomEvent::RoundStarted { round, deadline_ms } = event {
            Some((*round, *deadline_ms))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use play_room_core::{Move, RoomCommand};

    #[test]
    fn creates_hosted_room() {
        let owner_id = PlayerId::new("alice");
        let (room_id, room) = create_room(
            &owner_id,
            "testroom",
            GameRules::default(),
            "Alice".to_owned(),
        )
        .unwrap();
        let snapshot = room.snapshot();

        assert_eq!(snapshot.id, room_id);
        assert_eq!(snapshot.name, "testroom");
        assert!(snapshot.players.iter().any(|player| {
            player.id == owner_id
                && player.name == "Alice"
                && player.role == PlayerRole::Participant
        }));
    }

    #[test]
    fn detects_conflicting_display_name() {
        let host_id = PlayerId::new("alice");
        let (_room_id, room) = create_room(
            &host_id,
            "testroom",
            GameRules::default(),
            "Alice".to_owned(),
        )
        .unwrap();

        let conflict = player_name_conflict(&room, &PlayerId::new("other"), "alice").unwrap();

        assert_eq!(conflict.name, "Alice");
        assert!(conflict.connected);
        assert!(player_name_conflict(&room, &host_id, "alice").is_none());
    }

    #[test]
    fn extracts_round_timer_from_events() {
        let host_id = PlayerId::new("alice");
        let guest_id = PlayerId::new("bob");
        let (_room_id, mut room) = create_room(
            &host_id,
            "testroom",
            GameRules::default(),
            "Alice".to_owned(),
        )
        .unwrap();
        room.apply(RoomCommand::Join {
            player: Player::participant(guest_id.clone(), "Bob"),
        })
        .unwrap();
        room.apply(RoomCommand::SetReady {
            player_id: host_id,
            ready: true,
            now_ms: 1_000,
        })
        .unwrap();
        let events = room
            .apply(RoomCommand::SetReady {
                player_id: guest_id,
                ready: true,
                now_ms: 1_000,
            })
            .unwrap();

        let timer = round_timer(&events).unwrap();
        assert_eq!(timer.0, 1);
        assert!(timer.1 > 1_000);

        let no_timer = room
            .apply(RoomCommand::SubmitMove {
                player_id: PlayerId::new("bob"),
                mv: Move::Rock,
                now_ms: 1_100,
            })
            .unwrap();
        assert!(round_timer(&no_timer).is_none());
    }
}
