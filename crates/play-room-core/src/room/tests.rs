
use super::*;

fn two_player_room() -> GameRoom {
    let host = Player::participant(PlayerId::new("alice"), "Alice");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
    room.apply(RoomCommand::Join {
        player: Player::participant(PlayerId::new("bob"), "Bob"),
    })
    .unwrap();
    room
}

fn ready(room: &mut GameRoom, player_id: &str, now_ms: u64) {
    room.apply(RoomCommand::SetReady {
        player_id: PlayerId::new(player_id),
        ready: true,
        now_ms,
    })
    .unwrap();
}

fn alice_wins_round(room: &mut GameRoom, now_ms: u64) {
    ready(room, "alice", now_ms);
    ready(room, "bob", now_ms);
    room.apply(RoomCommand::SubmitMove {
        player_id: PlayerId::new("alice"),
        mv: Move::Paper,
        now_ms: now_ms + 1,
    })
    .unwrap();
    room.apply(RoomCommand::SubmitMove {
        player_id: PlayerId::new("bob"),
        mv: Move::Rock,
        now_ms: now_ms + 2,
    })
    .unwrap();
}

#[test]
fn scoreboard_excludes_spectators_but_keeps_disconnected_participants() {
    let host_id = PlayerId::new("host");
    let guest_id = PlayerId::new("guest");
    let spectator_id = PlayerId::new("spectator");
    let host = Player::participant(host_id.clone(), "Host");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

    room.apply(RoomCommand::Join {
        player: Player::participant(guest_id.clone(), "Guest"),
    })
    .unwrap();
    room.apply(RoomCommand::Join {
        player: Player::spectator(spectator_id.clone(), "Spectator"),
    })
    .unwrap();
    room.apply(RoomCommand::Disconnect {
        player_id: guest_id.clone(),
    })
    .unwrap();

    let scores = room.scoreboard();

    assert!(scores.iter().any(|score| score.player_id == host_id));
    assert!(scores.iter().any(|score| score.player_id == guest_id));
    assert!(!scores.iter().any(|score| score.player_id == spectator_id));
}

#[test]
fn duplicate_player_names_are_rejected_within_a_room() {
    let host = Player::participant(PlayerId::new("host"), "Alex");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

    let err = room
        .apply(RoomCommand::Join {
            player: Player::participant(PlayerId::new("guest"), "alex"),
        })
        .unwrap_err();

    assert_eq!(err, CoreError::DuplicatePlayerName("alex".to_owned()));
}

#[test]
fn duplicate_names_are_rejected_across_participants_and_spectators() {
    let host = Player::participant(PlayerId::new("host"), "Alex");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

    let spectator_err = room
        .apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("watcher"), "alex"),
        })
        .unwrap_err();
    assert_eq!(
        spectator_err,
        CoreError::DuplicatePlayerName("alex".to_owned())
    );

    room.apply(RoomCommand::Join {
        player: Player::spectator(PlayerId::new("mira"), "Mira"),
    })
    .unwrap();

    let participant_err = room
        .apply(RoomCommand::Join {
            player: Player::participant(PlayerId::new("guest"), "mira"),
        })
        .unwrap_err();
    assert_eq!(
        participant_err,
        CoreError::DuplicatePlayerName("mira".to_owned())
    );
}

#[test]
fn duplicate_names_are_rejected_between_spectators() {
    let host = Player::participant(PlayerId::new("host"), "Host");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
    room.apply(RoomCommand::Join {
        player: Player::spectator(PlayerId::new("watcher-one"), "Mira"),
    })
    .unwrap();

    let err = room
        .apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("watcher-two"), "mira"),
        })
        .unwrap_err();

    assert_eq!(err, CoreError::DuplicatePlayerName("mira".to_owned()));
}

#[test]
fn player_can_be_renamed_within_room() {
    let mut room = two_player_room();

    let events = room
        .apply(RoomCommand::RenamePlayer {
            player_id: PlayerId::new("alice"),
            name: "  Alicia  ".to_owned(),
        })
        .unwrap();
    let snapshot = room.snapshot();
    let alice = snapshot
        .players
        .iter()
        .find(|player| player.id == PlayerId::new("alice"))
        .unwrap();

    assert_eq!(
        events,
        vec![RoomEvent::PlayerRenamed {
            player_id: PlayerId::new("alice"),
            name: "Alicia".to_owned(),
        }]
    );
    assert_eq!(alice.name, "Alicia");
    assert_eq!(snapshot.scoreboard[0].name, "Alicia");
}

#[test]
fn rename_rejects_duplicate_or_empty_name() {
    let mut room = two_player_room();

    let duplicate = room
        .apply(RoomCommand::RenamePlayer {
            player_id: PlayerId::new("alice"),
            name: " bob ".to_owned(),
        })
        .unwrap_err();
    let empty = room
        .apply(RoomCommand::RenamePlayer {
            player_id: PlayerId::new("alice"),
            name: "  ".to_owned(),
        })
        .unwrap_err();

    assert_eq!(duplicate, CoreError::DuplicatePlayerName("bob".to_owned()));
    assert_eq!(empty, CoreError::EmptyName);
}

#[test]
fn current_host_can_update_race_target_before_match_starts() {
    let mut room = two_player_room();

    let events = room
        .apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("alice"),
            target_score: 3,
        })
        .unwrap();

    assert_eq!(room.snapshot().rules.target_score, 3);
    assert_eq!(
        events,
        vec![RoomEvent::MatchFormatChanged { target_score: 3 }]
    );
}

#[test]
fn race_target_update_rejects_unsupported_targets() {
    let mut room = two_player_room();

    let err = room
        .apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("alice"),
            target_score: 5,
        })
        .unwrap_err();

    assert_eq!(
        err,
        CoreError::InvalidRules("target_score must be one of 1, 2, or 3".to_owned())
    );
}

#[test]
fn updating_race_target_clears_ready_players() {
    let host = Player::participant(PlayerId::new("alice"), "Alice");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

    room.apply(RoomCommand::SetReady {
        player_id: PlayerId::new("alice"),
        ready: true,
        now_ms: 1_000,
    })
    .unwrap();
    let events = room
        .apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("alice"),
            target_score: 3,
        })
        .unwrap();
    let alice = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == PlayerId::new("alice"))
        .unwrap();

    assert!(!alice.ready);
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::ReadyChanged { player_id, ready }
            if player_id == &PlayerId::new("alice") && !ready
    )));
}

#[test]
fn non_host_cannot_update_race_target() {
    let mut room = two_player_room();

    let err = room
        .apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("bob"),
            target_score: 3,
        })
        .unwrap_err();

    assert_eq!(err, CoreError::HostOnly);
}

#[test]
fn transferred_host_can_update_race_target() {
    let mut room = two_player_room();

    room.apply(RoomCommand::Leave {
        player_id: PlayerId::new("alice"),
    })
    .unwrap();
    room.apply(RoomCommand::UpdateMatchFormat {
        player_id: PlayerId::new("bob"),
        target_score: 3,
    })
    .unwrap();

    assert_eq!(room.snapshot().host_id, Some(PlayerId::new("bob")));
    assert_eq!(room.snapshot().rules.target_score, 3);
}

#[test]
fn race_target_cannot_change_during_unfinished_match() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1_000);

    let err = room
        .apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("alice"),
            target_score: 3,
        })
        .unwrap_err();

    assert_eq!(err, CoreError::MatchInProgress);
}

#[test]
fn setting_ready_to_existing_value_is_noop() {
    let host = Player::participant(PlayerId::new("alice"), "Alice");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

    room.apply(RoomCommand::SetReady {
        player_id: PlayerId::new("alice"),
        ready: true,
        now_ms: 1_000,
    })
    .unwrap();
    let events = room
        .apply(RoomCommand::SetReady {
            player_id: PlayerId::new("alice"),
            ready: true,
            now_ms: 1_001,
        })
        .unwrap();

    assert!(events.is_empty());
}

#[test]
fn setting_existing_spectator_role_is_noop() {
    let host = Player::participant(PlayerId::new("alice"), "Alice");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
    room.apply(RoomCommand::Join {
        player: Player::spectator(PlayerId::new("mira"), "Mira"),
    })
    .unwrap();

    let events = room
        .apply(RoomCommand::SetSpectator {
            player_id: PlayerId::new("mira"),
            spectator: true,
        })
        .unwrap();

    assert!(events.is_empty());
}

#[test]
fn disconnected_player_cannot_switch_role() {
    let host = Player::participant(PlayerId::new("alice"), "Alice");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
    room.apply(RoomCommand::Disconnect {
        player_id: PlayerId::new("alice"),
    })
    .unwrap();

    let err = room
        .apply(RoomCommand::SetSpectator {
            player_id: PlayerId::new("alice"),
            spectator: true,
        })
        .unwrap_err();

    assert_eq!(err, CoreError::PlayerDisconnected);
}

#[test]
fn host_can_update_race_target_after_match_finishes() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1_000);
    alice_wins_round(&mut room, 2_000);

    room.apply(RoomCommand::UpdateMatchFormat {
        player_id: PlayerId::new("alice"),
        target_score: 3,
    })
    .unwrap();

    assert_eq!(room.snapshot().rules.target_score, 3);
}

#[test]
fn finished_match_rejects_ready_and_moves_until_host_resets() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);
    alice_wins_round(&mut room, 2000);

    let ready_err = room
        .apply(RoomCommand::SetReady {
            player_id: PlayerId::new("alice"),
            ready: true,
            now_ms: 3000,
        })
        .unwrap_err();
    assert_eq!(ready_err, CoreError::RoomFinished);

    let move_err = room
        .apply(RoomCommand::SubmitMove {
            player_id: PlayerId::new("alice"),
            mv: Move::Rock,
            now_ms: 3001,
        })
        .unwrap_err();
    assert_eq!(move_err, CoreError::RoundNotActive);

    room.apply(RoomCommand::StartNextMatch {
        player_id: PlayerId::new("alice"),
    })
    .unwrap();
    ready(&mut room, "alice", 4000);
    ready(&mut room, "bob", 4000);

    assert!(matches!(room.phase(), RoomPhase::InRound { round: 1, .. }));
}

#[test]
fn submit_move_before_deadline_is_accepted() {
    let mut room = two_player_room();
    ready(&mut room, "alice", 1000);
    ready(&mut room, "bob", 1000);
    let deadline_ms = match room.phase() {
        RoomPhase::InRound { deadline_ms, .. } => *deadline_ms,
        _ => panic!("round should be active"),
    };

    let events = room
        .apply(RoomCommand::SubmitMove {
            player_id: PlayerId::new("alice"),
            mv: Move::Rock,
            now_ms: deadline_ms - 1,
        })
        .unwrap();

    assert_eq!(room.moves.get(&PlayerId::new("alice")), Some(&Move::Rock));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::MoveAccepted { player_id } if player_id == &PlayerId::new("alice")
    )));
}

#[test]
fn submit_move_at_or_after_deadline_is_rejected() {
    let mut room = two_player_room();
    ready(&mut room, "alice", 1000);
    ready(&mut room, "bob", 1000);
    let (round, deadline_ms) = match room.phase() {
        RoomPhase::InRound { round, deadline_ms } => (*round, *deadline_ms),
        _ => panic!("round should be active"),
    };

    let err = room
        .apply(RoomCommand::SubmitMove {
            player_id: PlayerId::new("alice"),
            mv: Move::Rock,
            now_ms: deadline_ms,
        })
        .unwrap_err();

    assert_eq!(err, CoreError::RoundExpired);
    assert!(!room.moves.contains_key(&PlayerId::new("alice")));
    room.apply(RoomCommand::TimeoutRound {
        round,
        now_ms: deadline_ms,
    })
    .unwrap();
    assert!(matches!(room.phase(), RoomPhase::Lobby));
}

#[test]
fn participant_seat_expiry_demotes_disconnected_player_to_spectator() {
    let mut room = two_player_room();

    room.apply(RoomCommand::Disconnect {
        player_id: PlayerId::new("alice"),
    })
    .unwrap();
    let events = room
        .apply(RoomCommand::ExpireParticipantSeat {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
    let snapshot = room.snapshot();
    let alice = snapshot
        .players
        .iter()
        .find(|player| player.id == PlayerId::new("alice"))
        .unwrap();

    assert_eq!(alice.role, PlayerRole::Spectator);
    assert!(!alice.connected);
    assert_eq!(snapshot.host_id, Some(PlayerId::new("bob")));
    assert!(!snapshot
        .scoreboard
        .iter()
        .any(|score| score.player_id == PlayerId::new("alice")));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::RoleChanged { player_id, role }
            if player_id == &PlayerId::new("alice") && role == &PlayerRole::Spectator
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::HostChanged { host_id } if host_id == &Some(PlayerId::new("bob"))
    )));
}

#[test]
fn participant_seat_expiry_is_noop_after_reconnect() {
    let mut room = two_player_room();

    room.apply(RoomCommand::Disconnect {
        player_id: PlayerId::new("alice"),
    })
    .unwrap();
    room.apply(RoomCommand::Reconnect {
        player_id: PlayerId::new("alice"),
    })
    .unwrap();
    let events = room
        .apply(RoomCommand::ExpireParticipantSeat {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
    let snapshot = room.snapshot();
    let alice = snapshot
        .players
        .iter()
        .find(|player| player.id == PlayerId::new("alice"))
        .unwrap();

    assert!(events.is_empty());
    assert_eq!(alice.role, PlayerRole::Participant);
    assert!(alice.connected);
    assert_eq!(snapshot.host_id, Some(PlayerId::new("alice")));
}

#[test]
fn race_to_two_finishes_when_a_player_reaches_two_points() {
    let mut room = two_player_room();

    alice_wins_round(&mut room, 1000);
    assert!(matches!(room.phase(), RoomPhase::Lobby));

    alice_wins_round(&mut room, 2000);

    assert!(matches!(
        room.phase(),
        RoomPhase::Finished {
            winner: Some(player_id)
        } if player_id == &PlayerId::new("alice")
    ));
    assert_eq!(room.snapshot().scoreboard[0].score, 2);
}

#[test]
fn role_switching_is_rejected_between_unfinished_rounds() {
    let mut room = two_player_room();
    room.apply(RoomCommand::Join {
        player: Player::spectator(PlayerId::new("mira"), "Mira"),
    })
    .unwrap();
    alice_wins_round(&mut room, 1000);

    let participant_err = room
        .apply(RoomCommand::SetSpectator {
            player_id: PlayerId::new("alice"),
            spectator: true,
        })
        .unwrap_err();
    let spectator_err = room
        .apply(RoomCommand::SetSpectator {
            player_id: PlayerId::new("mira"),
            spectator: false,
        })
        .unwrap_err();

    assert_eq!(participant_err, CoreError::MatchInProgress);
    assert_eq!(spectator_err, CoreError::MatchInProgress);
}

#[test]
fn participant_join_is_rejected_between_unfinished_rounds() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);

    let err = room
        .apply(RoomCommand::Join {
            player: Player::participant(PlayerId::new("carol"), "Carol"),
        })
        .unwrap_err();

    assert_eq!(err, CoreError::MatchInProgress);
}

#[test]
fn participant_leave_between_rounds_finishes_match_by_forfeit() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);

    let events = room
        .apply(RoomCommand::Leave {
            player_id: PlayerId::new("bob"),
        })
        .unwrap();

    assert!(matches!(
        room.phase(),
        RoomPhase::Finished {
            winner: Some(player_id)
        } if player_id == &PlayerId::new("alice")
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::PlayerLeft { player_id } if player_id == &PlayerId::new("bob")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::GameEnded { winner } if winner == &Some(PlayerId::new("alice"))
    )));
    assert_eq!(
        room.snapshot().scoreboard[0].player_id,
        PlayerId::new("alice")
    );
}

#[test]
fn participant_disconnect_during_round_finishes_match_by_forfeit() {
    let mut room = two_player_room();
    ready(&mut room, "alice", 1000);
    ready(&mut room, "bob", 1000);

    let events = room
        .apply(RoomCommand::Disconnect {
            player_id: PlayerId::new("bob"),
        })
        .unwrap();

    assert!(matches!(
        room.phase(),
        RoomPhase::Finished {
            winner: Some(player_id)
        } if player_id == &PlayerId::new("alice")
    ));
    assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::RoundResolved { result }
                if result.reason == RoundEndReason::PlayerLeft
                    && matches!(&result.outcome, RoundOutcome::Win { winner } if winner == &PlayerId::new("alice"))
        )));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::GameEnded { winner } if winner == &Some(PlayerId::new("alice"))
    )));
    assert_eq!(room.snapshot().scoreboard[0].score, 1);
}

#[test]
fn host_disconnect_during_round_transfers_host_and_forfeits_match() {
    let mut room = two_player_room();
    ready(&mut room, "alice", 1000);
    ready(&mut room, "bob", 1000);

    let events = room
        .apply(RoomCommand::Disconnect {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
    let snapshot = room.snapshot();

    assert!(matches!(
        room.phase(),
        RoomPhase::Finished {
            winner: Some(player_id)
        } if player_id == &PlayerId::new("bob")
    ));
    assert_eq!(snapshot.host_id, Some(PlayerId::new("bob")));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::HostChanged { host_id } if host_id == &Some(PlayerId::new("bob"))
    )));
    assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::RoundResolved { result }
                if result.reason == RoundEndReason::PlayerLeft
                    && matches!(&result.outcome, RoundOutcome::Win { winner } if winner == &PlayerId::new("bob"))
        )));
    assert!(snapshot
        .scoreboard
        .iter()
        .any(|score| score.player_id == PlayerId::new("bob") && score.score == 1));
}

#[test]
fn host_disconnect_between_rounds_transfers_host_and_forfeits_match() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);

    let events = room
        .apply(RoomCommand::Disconnect {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
    let snapshot = room.snapshot();

    assert!(matches!(
        room.phase(),
        RoomPhase::Finished {
            winner: Some(player_id)
        } if player_id == &PlayerId::new("bob")
    ));
    assert_eq!(snapshot.host_id, Some(PlayerId::new("bob")));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::HostChanged { host_id } if host_id == &Some(PlayerId::new("bob"))
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::GameEnded { winner } if winner == &Some(PlayerId::new("bob"))
    )));
}

#[test]
fn spectator_join_is_allowed_during_unfinished_match() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);

    let events = room
        .apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("mira"), "Mira"),
        })
        .unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::PlayerJoined { player_id, role, .. }
            if player_id == &PlayerId::new("mira") && role == &PlayerRole::Spectator
    )));
}

#[test]
fn host_can_reset_finished_match_without_changing_seats() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);
    alice_wins_round(&mut room, 2000);

    let events = room
        .apply(RoomCommand::StartNextMatch {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();

    assert!(matches!(room.phase(), RoomPhase::Lobby));
    assert!(matches!(events.as_slice(), [RoomEvent::MatchReset { .. }]));
    assert_eq!(room.snapshot().round, 0);
    assert!(room
        .snapshot()
        .players
        .iter()
        .all(|player| player.score == 0));
    assert!(room.snapshot().players.iter().all(|player| !player.ready));
    assert_eq!(room.participant_count(), 2);
}

#[test]
fn non_host_cannot_reset_finished_match() {
    let mut room = two_player_room();
    alice_wins_round(&mut room, 1000);
    alice_wins_round(&mut room, 2000);

    let err = room
        .apply(RoomCommand::StartNextMatch {
            player_id: PlayerId::new("bob"),
        })
        .unwrap_err();

    assert_eq!(err, CoreError::HostOnly);
}

#[test]
fn host_transfer_prefers_connected_participants() {
    let host = Player::participant(PlayerId::new("host"), "Host");
    let mut room = GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
    room.apply(RoomCommand::Join {
        player: Player::spectator(PlayerId::new("spectator"), "Spectator"),
    })
    .unwrap();
    room.apply(RoomCommand::Join {
        player: Player::participant(PlayerId::new("guest"), "Guest"),
    })
    .unwrap();

    room.apply(RoomCommand::Leave {
        player_id: PlayerId::new("host"),
    })
    .unwrap();

    assert_eq!(room.snapshot().host_id, Some(PlayerId::new("guest")));
}
