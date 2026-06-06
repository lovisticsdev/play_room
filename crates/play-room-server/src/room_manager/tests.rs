use super::*;
use play_room_protocol::ServerEvent;

fn connect_named(manager: &mut RoomManager, name: &str) -> ConnectedPlayer {
    let (tx, _) = crate::broadcast::channel();
    manager.connect(name.to_owned(), None, tx)
}

#[test]
fn joins_room_by_exact_name_when_id_is_not_used() {
    let mut manager = RoomManager::default();
    let host_id = PlayerId::new("host");
    let (room_id, _) = manager
        .create_room(&host_id, "testroom".to_owned(), None)
        .unwrap();
    let guest_id = PlayerId::new("guest");

    let messages = manager
        .join_room(&guest_id, &RoomId::new("TESTROOM"))
        .unwrap();

    assert!(!messages.is_empty());
    assert_eq!(manager.player_room(&guest_id), Some(&room_id));
}

#[test]
fn duplicate_room_names_are_rejected_with_suggestions() {
    let mut manager = RoomManager::default();
    manager
        .create_room(&PlayerId::new("host-one"), "testroom".to_owned(), None)
        .unwrap();

    let err = manager
        .create_room(&PlayerId::new("host-two"), "TestRoom".to_owned(), None)
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::RoomNameExists));
    assert_eq!(err.message(), "room name already exists: TestRoom");
    assert!(err.suggestions().iter().any(|name| name == "testroom-2"));
}

#[test]
fn can_spectate_a_full_room_by_exact_name() {
    let mut manager = RoomManager::default();
    let host_id = PlayerId::new("host");
    let (room_id, _) = manager
        .create_room(&host_id, "testroom".to_owned(), None)
        .unwrap();
    manager
        .join_room(&PlayerId::new("guest"), &RoomId::new("testroom"))
        .unwrap();

    let spectator_id = PlayerId::new("spectator");
    let messages = manager
        .spectate_room(&spectator_id, &RoomId::new("testroom"))
        .unwrap();

    let room = manager.rooms.get(&room_id).unwrap();
    let spectator = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == spectator_id)
        .unwrap();

    assert!(!messages.is_empty());
    assert_eq!(spectator.role, PlayerRole::Spectator);
}

#[test]
fn auto_enter_open_room_joins_as_participant() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();

    let messages = manager
        .enter_room(
            &bob.player_id,
            &RoomId::new("testroom"),
            EnterRoomMode::Auto,
        )
        .unwrap();
    let room = manager.rooms.get(&room_id).unwrap();
    let bob_view = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == bob.player_id)
        .unwrap();

    assert!(!messages.is_empty());
    assert_eq!(bob_view.role, PlayerRole::Participant);
}

#[test]
fn auto_enter_full_room_spectates() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let mira = connect_named(&mut manager, "Mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();

    let messages = manager
        .enter_room(&mira.player_id, &room_id, EnterRoomMode::Auto)
        .unwrap();
    let room = manager.rooms.get(&room_id).unwrap();
    let mira_view = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == mira.player_id)
        .unwrap();

    assert!(!messages.is_empty());
    assert_eq!(mira_view.role, PlayerRole::Spectator);
}

#[test]
fn auto_enter_active_match_spectates() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let mira = connect_named(&mut manager, "Mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();
    manager
        .apply_to_current_room(
            &alice.player_id,
            RoomCommand::SetReady {
                player_id: alice.player_id.clone(),
                ready: true,
                now_ms: 1_000,
            },
        )
        .unwrap();
    manager
        .apply_to_current_room(
            &bob.player_id,
            RoomCommand::SetReady {
                player_id: bob.player_id.clone(),
                ready: true,
                now_ms: 1_000,
            },
        )
        .unwrap();

    let messages = manager
        .enter_room(
            &mira.player_id,
            &RoomId::new("TESTROOM"),
            EnterRoomMode::Auto,
        )
        .unwrap();
    let room = manager.rooms.get(&room_id).unwrap();
    let mira_view = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == mira.player_id)
        .unwrap();

    assert!(!messages.is_empty());
    assert_eq!(mira_view.role, PlayerRole::Spectator);
}

#[test]
fn duplicate_disconnected_player_name_is_rejected_with_clear_message() {
    let mut manager = RoomManager::default();
    let (alice_tx, _) = crate::broadcast::channel();
    let alice = manager.connect("Alice".to_owned(), None, alice_tx);
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.disconnect(&alice.player_id, alice.connection_id, 1_000);

    let (other_tx, _) = crate::broadcast::channel();
    let other = manager.connect("alice".to_owned(), None, other_tx);
    let err = manager.join_room(&other.player_id, &room_id).unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
    assert!(err.message().contains("currently disconnected"));
    assert_eq!(err.suggestions(), ["Alice-2", "Alice-3", "Alice-4"]);
}

#[test]
fn duplicate_connected_player_name_is_rejected_with_suggestions() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let other_alice = connect_named(&mut manager, "alice");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();

    let err = manager
        .join_room(&other_alice.player_id, &room_id)
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
    assert_eq!(err.suggestions(), ["Alice-2", "Alice-3", "Alice-4"]);
}

#[test]
fn update_display_name_renames_session_and_current_room() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();

    let messages = manager
        .update_display_name(&bob.player_id, "  Bobby  ".to_owned())
        .unwrap();
    let bob_view = manager
        .rooms
        .get(&room_id)
        .unwrap()
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == bob.player_id)
        .unwrap();

    assert_eq!(
        manager.session_registry.player_name(&bob.player_id),
        Some("Bobby")
    );
    assert_eq!(bob_view.name, "Bobby");
    assert!(has_room_event(
        &messages,
        &alice.player_id,
        &room_id,
        |event| matches!(
            event,
            RoomEvent::PlayerRenamed { player_id, name }
                if *player_id == bob.player_id && name == "Bobby"
        )
    ));
}

#[test]
fn update_display_name_rejects_duplicate_current_room_name() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();

    let err = manager
        .update_display_name(&bob.player_id, " alice ".to_owned())
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
    assert_eq!(err.suggestions(), ["Alice-2", "Alice-3", "Alice-4"]);
    assert_eq!(
        manager.session_registry.player_name(&bob.player_id),
        Some("Bob")
    );
}

#[test]
fn update_display_name_without_room_updates_retained_session_name() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");

    let messages = manager
        .update_display_name(&alice.player_id, "Alicia".to_owned())
        .unwrap();

    assert!(messages.is_empty());
    assert_eq!(
        manager.session_registry.player_name(&alice.player_id),
        Some("Alicia")
    );
}

fn has_room_event(
    messages: &OutboundMessages,
    recipient: &PlayerId,
    room_id: &RoomId,
    predicate: impl Fn(&RoomEvent) -> bool,
) -> bool {
    messages.iter().any(|(target, message)| {
        target == recipient
            && matches!(
                message,
                ServerMessage::Event {
                    event: ServerEvent::RoomEvent {
                        room_id: event_room_id,
                        event
                    }
                } if event_room_id == room_id && predicate(event)
            )
    })
}

fn has_snapshot_without_player(
    messages: &OutboundMessages,
    recipient: &PlayerId,
    room_id: &RoomId,
    absent_player_id: &PlayerId,
) -> bool {
    messages.iter().any(|(target, message)| {
        target == recipient
            && matches!(
                message,
                ServerMessage::Event {
                    event: ServerEvent::RoomSnapshot { room }
                } if &room.id == room_id
                    && !room.players.iter().any(|player| &player.id == absent_player_id)
            )
    })
}

fn room_has_player(manager: &RoomManager, room_id: &RoomId, player_id: &PlayerId) -> bool {
    manager
        .rooms
        .get(room_id)
        .map(|room| {
            room.snapshot()
                .players
                .iter()
                .any(|player| &player.id == player_id)
        })
        .unwrap_or(false)
}

#[test]
fn create_room_preserves_old_room_leave_messages() {
    let mut manager = RoomManager::default();
    let alice = PlayerId::new("alice");
    let bob = PlayerId::new("bob");
    let (old_room_id, _) = manager
        .create_room(&alice, "old-room".to_owned(), None)
        .unwrap();
    manager.join_room(&bob, &old_room_id).unwrap();

    let (new_room_id, messages) = manager
        .create_room(&alice, "new-room".to_owned(), None)
        .unwrap();

    assert_eq!(manager.player_room(&alice), Some(&new_room_id));
    assert!(has_room_event(
        &messages,
        &bob,
        &old_room_id,
        |event| matches!(
            event,
            RoomEvent::PlayerLeft { player_id } if player_id == &alice
        )
    ));
    assert!(has_snapshot_without_player(
        &messages,
        &bob,
        &old_room_id,
        &alice
    ));
}

#[test]
fn joining_another_room_preserves_old_room_leave_messages() {
    let mut manager = RoomManager::default();
    let alice = PlayerId::new("alice");
    let bob = PlayerId::new("bob");
    let carol = PlayerId::new("carol");
    let (old_room_id, _) = manager
        .create_room(&alice, "old-room".to_owned(), None)
        .unwrap();
    manager.join_room(&bob, &old_room_id).unwrap();
    let (new_room_id, _) = manager
        .create_room(&carol, "new-room".to_owned(), None)
        .unwrap();

    let messages = manager.join_room(&alice, &new_room_id).unwrap();

    assert_eq!(manager.player_room(&alice), Some(&new_room_id));
    assert!(has_room_event(
        &messages,
        &bob,
        &old_room_id,
        |event| matches!(
            event,
            RoomEvent::PlayerLeft { player_id } if player_id == &alice
        )
    ));
    assert!(has_snapshot_without_player(
        &messages,
        &bob,
        &old_room_id,
        &alice
    ));
}

#[test]
fn leaving_room_releases_display_name_for_that_room() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();

    manager.leave_current_room(&alice.player_id).unwrap();

    assert!(!room_has_player(&manager, &room_id, &alice.player_id));
    let other_alice = connect_named(&mut manager, "alice");
    let messages = manager.join_room(&other_alice.player_id, &room_id).unwrap();

    assert!(!messages.is_empty());
    assert!(room_has_player(&manager, &room_id, &other_alice.player_id));
}

#[test]
fn player_can_return_to_previous_room_after_moving_away() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let carol = connect_named(&mut manager, "Carol");
    let (old_room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &old_room_id).unwrap();
    let (new_room_id, _) = manager
        .create_room(&carol.player_id, "otherroom".to_owned(), None)
        .unwrap();

    manager.join_room(&alice.player_id, &new_room_id).unwrap();
    assert!(!room_has_player(&manager, &old_room_id, &alice.player_id));

    manager.join_room(&alice.player_id, &old_room_id).unwrap();

    assert_eq!(manager.player_room(&alice.player_id), Some(&old_room_id));
    assert!(room_has_player(&manager, &old_room_id, &alice.player_id));
    assert!(!room_has_player(&manager, &new_room_id, &alice.player_id));
}

#[test]
fn moving_between_rooms_releases_display_name_in_previous_room() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let carol = connect_named(&mut manager, "Carol");
    let (old_room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &old_room_id).unwrap();
    let (new_room_id, _) = manager
        .create_room(&carol.player_id, "otherroom".to_owned(), None)
        .unwrap();

    manager.join_room(&alice.player_id, &new_room_id).unwrap();
    let other_alice = connect_named(&mut manager, "alice");
    let messages = manager
        .join_room(&other_alice.player_id, &old_room_id)
        .unwrap();

    assert!(!messages.is_empty());
    assert!(room_has_player(&manager, &new_room_id, &alice.player_id));
    assert!(room_has_player(
        &manager,
        &old_room_id,
        &other_alice.player_id
    ));
}

#[test]
fn spectator_names_are_checked_against_all_room_members() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let mira = connect_named(&mut manager, "Mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();

    let alice_clone = connect_named(&mut manager, "alice");
    let participant_conflict = manager
        .spectate_room(&alice_clone.player_id, &room_id)
        .unwrap_err();
    assert_eq!(
        participant_conflict.code(),
        Some(&ErrorCode::PlayerNameExists)
    );
    assert_eq!(
        participant_conflict.suggestions(),
        ["Alice-2", "Alice-3", "Alice-4"]
    );

    manager.spectate_room(&mira.player_id, &room_id).unwrap();
    let mira_clone = connect_named(&mut manager, "mira");
    let spectator_conflict = manager
        .spectate_room(&mira_clone.player_id, &room_id)
        .unwrap_err();

    assert_eq!(
        spectator_conflict.code(),
        Some(&ErrorCode::PlayerNameExists)
    );
    assert_eq!(
        spectator_conflict.suggestions(),
        ["Mira-2", "Mira-3", "Mira-4"]
    );
}

#[test]
fn disconnected_spectator_gets_name_expiry_in_snapshot() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let mira = connect_named(&mut manager, "Mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.spectate_room(&mira.player_id, &room_id).unwrap();

    let outcome = manager.disconnect(&mira.player_id, mira.connection_id, 5_000);
    let expiry = outcome.spectator_expiry.unwrap();
    let snapshot = manager.room_snapshot(&room_id).unwrap();

    assert!(outcome.seat_expiry.is_none());
    assert_eq!(expiry.expires_at_ms, 65_000);
    assert!(snapshot.players.iter().any(|player| {
        player.id == mira.player_id && player.spectator_expires_at_ms == Some(65_000)
    }));
}

#[test]
fn spectator_name_expiry_removes_disconnected_spectator_and_frees_name() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let mira = connect_named(&mut manager, "Mira");
    let other_mira = connect_named(&mut manager, "mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.spectate_room(&mira.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&mira.player_id, mira.connection_id, 1_000)
        .spectator_expiry
        .unwrap();

    let err = manager
        .spectate_room(&other_mira.player_id, &room_id)
        .unwrap_err();
    assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
    assert_eq!(err.suggestions(), ["Mira-2", "Mira-3", "Mira-4"]);

    let messages = manager.expire_spectator(&expiry).unwrap();
    let join_messages = manager
        .spectate_room(&other_mira.player_id, &room_id)
        .unwrap();

    assert!(!messages.is_empty());
    assert!(!room_has_player(&manager, &room_id, &mira.player_id));
    assert!(!join_messages.is_empty());
}

#[test]
fn spectator_name_expiry_is_ignored_after_reconnect() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let mira = connect_named(&mut manager, "Mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.spectate_room(&mira.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&mira.player_id, mira.connection_id, 1_000)
        .spectator_expiry
        .unwrap();

    let (tx, _) = crate::broadcast::channel();
    manager.connect(String::new(), Some(mira.reconnect_token.clone()), tx);
    let messages = manager.expire_spectator(&expiry).unwrap();
    let snapshot = manager.room_snapshot(&room_id).unwrap();
    let mira_view = snapshot
        .players
        .iter()
        .find(|player| player.id == mira.player_id)
        .unwrap();

    assert!(messages.is_empty());
    assert_eq!(mira_view.role, PlayerRole::Spectator);
    assert!(mira_view.connected);
}
#[test]
fn disconnected_participant_keeps_seat_until_expiry() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let carol = connect_named(&mut manager, "Carol");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();

    let outcome = manager.disconnect(&alice.player_id, alice.connection_id, 1_000);
    let err = manager.join_room(&carol.player_id, &room_id).unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::RoomFull));
    assert_eq!(
        outcome
            .seat_expiry
            .as_ref()
            .map(|expiry| expiry.expires_at_ms),
        Some(31_000)
    );
    assert!(room_has_player(&manager, &room_id, &alice.player_id));
}

#[test]
fn disconnect_snapshot_includes_authoritative_seat_expiry() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();

    let outcome = manager.disconnect(&alice.player_id, alice.connection_id, 1_000);
    let expiry = outcome.seat_expiry.unwrap();

    assert!(outcome.messages.iter().any(|(_, message)| matches!(
        message,
        ServerMessage::Event {
            event: ServerEvent::RoomSnapshot { room }
        } if room.players.iter().any(|player|
            player.id == alice.player_id
                && player.participant_seat_expires_at_ms == Some(expiry.expires_at_ms)
        )
    )));
}
#[test]
fn seat_expiry_demotes_disconnected_participant_and_frees_slot() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let mira = connect_named(&mut manager, "Mira");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();
    manager.spectate_room(&mira.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&alice.player_id, alice.connection_id, 1_000)
        .seat_expiry
        .unwrap();

    let outcome = manager.expire_participant_seat(&expiry).unwrap();
    let (_, events, _) = manager
        .apply_to_current_room(
            &mira.player_id,
            RoomCommand::SetSpectator {
                player_id: mira.player_id.clone(),
                spectator: false,
            },
        )
        .unwrap();
    let room = manager.rooms.get(&room_id).unwrap();
    let snapshot = room.snapshot();
    let alice_view = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.player_id)
        .unwrap();
    let mira_view = snapshot
        .players
        .iter()
        .find(|player| player.id == mira.player_id)
        .unwrap();

    assert!(!outcome.messages.is_empty());
    assert!(outcome.spectator_expiry.is_some());
    assert_eq!(alice_view.role, PlayerRole::Spectator);
    assert!(!alice_view.connected);
    assert_eq!(mira_view.role, PlayerRole::Participant);
    assert!(events.iter().any(|event| matches!(
        event,
        RoomEvent::RoleChanged { player_id, role }
            if player_id == &mira.player_id && role == &PlayerRole::Participant
    )));
}

#[test]
fn seat_expiry_keeps_display_name_reserved_until_spectator_expiry() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let other_alice = connect_named(&mut manager, "alice");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&alice.player_id, alice.connection_id, 1_000)
        .seat_expiry
        .unwrap();
    let outcome = manager.expire_participant_seat(&expiry).unwrap();
    let spectator_expiry = outcome.spectator_expiry.unwrap();

    let err = manager
        .spectate_room(&other_alice.player_id, &room_id)
        .unwrap_err();
    assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
    assert!(err.message().contains("currently disconnected"));
    assert_eq!(err.suggestions(), ["Alice-2", "Alice-3", "Alice-4"]);

    manager.expire_spectator(&spectator_expiry).unwrap();
    let messages = manager
        .spectate_room(&other_alice.player_id, &room_id)
        .unwrap();

    assert_eq!(spectator_expiry.expires_at_ms, 91_000);
    assert!(!room_has_player(&manager, &room_id, &alice.player_id));
    assert!(!messages.is_empty());
}
#[test]
fn expired_player_reconnects_as_spectator() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&alice.player_id, alice.connection_id, 1_000)
        .seat_expiry
        .unwrap();
    let outcome = manager.expire_participant_seat(&expiry).unwrap();
    assert!(outcome.spectator_expiry.is_some());

    let (tx, _) = crate::broadcast::channel();
    let reconnected = manager.connect(String::new(), Some(alice.reconnect_token.clone()), tx);
    let room = manager.rooms.get(&room_id).unwrap();
    let alice_view = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == alice.player_id)
        .unwrap();

    assert_eq!(reconnected.player_id, alice.player_id);
    assert!(reconnected.reconnected);
    assert!(!reconnected.stale_token_replaced);
    assert!(reconnected.room_restored);
    assert_eq!(alice_view.role, PlayerRole::Spectator);
    assert!(alice_view.connected);
}

#[test]
fn seat_expiry_is_ignored_after_reconnect() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&alice.player_id, alice.connection_id, 1_000)
        .seat_expiry
        .unwrap();
    let (tx, _) = crate::broadcast::channel();
    manager.connect(String::new(), Some(alice.reconnect_token.clone()), tx);

    let outcome = manager.expire_participant_seat(&expiry).unwrap();
    let room = manager.rooms.get(&room_id).unwrap();
    let alice_view = room
        .snapshot()
        .players
        .into_iter()
        .find(|player| player.id == alice.player_id)
        .unwrap();

    assert!(outcome.messages.is_empty());
    assert!(outcome.spectator_expiry.is_none());
    assert_eq!(alice_view.role, PlayerRole::Participant);
    assert!(alice_view.connected);
}

#[test]
fn seat_expiry_is_ignored_after_leave() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    let bob = connect_named(&mut manager, "Bob");
    let (room_id, _) = manager
        .create_room(&alice.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.join_room(&bob.player_id, &room_id).unwrap();
    let expiry = manager
        .disconnect(&alice.player_id, alice.connection_id, 1_000)
        .seat_expiry
        .unwrap();
    manager.leave_current_room(&alice.player_id).unwrap();

    let outcome = manager.expire_participant_seat(&expiry).unwrap();

    assert!(outcome.messages.is_empty());
    assert!(outcome.spectator_expiry.is_none());
    assert!(!room_has_player(&manager, &room_id, &alice.player_id));
}

#[test]
fn reconnect_without_room_restores_identity_without_room() {
    let mut manager = RoomManager::default();
    let alice = connect_named(&mut manager, "Alice");
    manager.disconnect(&alice.player_id, alice.connection_id, 1_000);

    let (tx, _) = crate::broadcast::channel();
    let reconnected = manager.connect(String::new(), Some(alice.reconnect_token.clone()), tx);

    assert_eq!(reconnected.player_id, alice.player_id);
    assert!(reconnected.reconnected);
    assert!(!reconnected.stale_token_replaced);
    assert!(!reconnected.room_restored);
    assert!(reconnected.messages.is_empty());
}

#[test]
fn reconnect_returns_room_snapshot_to_reconnecting_player() {
    let mut manager = RoomManager::default();
    let (tx, _) = crate::broadcast::channel();
    let connected = manager.connect("alice".to_owned(), None, tx);
    let (room_id, _) = manager
        .create_room(&connected.player_id, "testroom".to_owned(), None)
        .unwrap();
    manager.disconnect(&connected.player_id, connected.connection_id, 1_000);

    let (reconnect_tx, _) = crate::broadcast::channel();
    let reconnected = manager.connect(
        String::new(),
        Some(connected.reconnect_token.clone()),
        reconnect_tx,
    );

    assert_eq!(reconnected.player_id, connected.player_id);
    assert!(reconnected.reconnected);
    assert!(!reconnected.stale_token_replaced);
    assert!(reconnected.room_restored);
    assert!(reconnected.messages.iter().any(|(target, message)| {
        target == &connected.player_id
            && matches!(
                message,
                ServerMessage::Event {
                    event: ServerEvent::RoomSnapshot { room }
                } if room.id == room_id
                    && room
                        .players
                        .iter()
                        .any(|player| player.id == connected.player_id && player.connected)
            )
    }));
}

#[test]
fn stale_disconnect_after_reconnect_does_not_mark_room_player_disconnected() {
    let mut manager = RoomManager::default();
    let (tx, _) = crate::broadcast::channel();
    let connected = manager.connect("alice".to_owned(), None, tx);
    let (room_id, _) = manager
        .create_room(&connected.player_id, "testroom".to_owned(), None)
        .unwrap();

    let (reconnect_tx, _) = crate::broadcast::channel();
    let reconnected = manager.connect(
        String::new(),
        Some(connected.reconnect_token.clone()),
        reconnect_tx,
    );
    let outcome = manager.disconnect(&connected.player_id, connected.connection_id, 2_000);
    let snapshot = manager.room_snapshot(&room_id).unwrap();
    let player = snapshot
        .players
        .iter()
        .find(|player| player.id == connected.player_id)
        .unwrap();

    assert_eq!(reconnected.player_id, connected.player_id);
    assert_ne!(reconnected.connection_id, connected.connection_id);
    assert!(outcome.messages.is_empty());
    assert!(outcome.seat_expiry.is_none());
    assert!(outcome.spectator_expiry.is_none());
    assert!(player.connected);
    assert_eq!(manager.session_registry.active_count(), 1);
}
#[test]
fn failed_move_to_full_room_keeps_player_in_current_room() {
    let mut manager = RoomManager::default();
    let alice = PlayerId::new("alice");
    let bob = PlayerId::new("bob");
    let carol = PlayerId::new("carol");
    let dave = PlayerId::new("dave");
    let (old_room_id, _) = manager
        .create_room(&alice, "old-room".to_owned(), None)
        .unwrap();
    manager.join_room(&bob, &old_room_id).unwrap();
    let (full_room_id, _) = manager
        .create_room(&carol, "full-room".to_owned(), None)
        .unwrap();
    manager.join_room(&dave, &full_room_id).unwrap();

    let err = manager.join_room(&alice, &full_room_id).unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::RoomFull));
    assert_eq!(manager.player_room(&alice), Some(&old_room_id));
    assert!(room_has_player(&manager, &old_room_id, &alice));
    assert!(!room_has_player(&manager, &full_room_id, &alice));
}
#[test]
fn unknown_reconnect_token_starts_new_session_with_notice() {
    let mut manager = RoomManager::default();
    let (tx, _) = crate::broadcast::channel();
    let missing = SessionToken::new("missing-token");

    let connected = manager.connect("   ".to_owned(), Some(missing.clone()), tx);

    assert_ne!(connected.reconnect_token, missing);
    assert!(!connected.reconnected);
    assert!(connected.stale_token_replaced);
    assert!(!connected.room_restored);
    assert_eq!(
        manager.session_registry.player_name(&connected.player_id),
        Some("Guest")
    );
    assert!(connected.messages.iter().any(|(target, message)| {
        target == &connected.player_id
            && matches!(
                message,
                ServerMessage::Event {
                    event: ServerEvent::Notice { message }
                } if message.contains("not recognized")
            )
    }));
}

#[test]
fn max_rooms_rejects_new_room_when_limit_is_reached() {
    let mut manager = RoomManager::new(RoomManagerLimits {
        max_rooms: 1,
        ..RoomManagerLimits::default()
    });
    manager
        .create_room(&PlayerId::new("host-one"), "one".to_owned(), None)
        .unwrap();

    let err = manager
        .create_room(&PlayerId::new("host-two"), "two".to_owned(), None)
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::RoomLimitReached));
}

#[test]
fn max_rooms_allows_replacing_own_single_member_room() {
    let mut manager = RoomManager::new(RoomManagerLimits {
        max_rooms: 1,
        ..RoomManagerLimits::default()
    });
    let alice = PlayerId::new("alice");
    let (old_room_id, _) = manager.create_room(&alice, "old".to_owned(), None).unwrap();

    let (new_room_id, _) = manager.create_room(&alice, "new".to_owned(), None).unwrap();

    assert_ne!(new_room_id, old_room_id);
    assert!(manager.rooms.get(&old_room_id).is_none());
    assert_eq!(manager.rooms.len(), 1);
}

#[test]
fn max_clients_rejects_new_identity_when_retained_session_limit_is_reached() {
    let mut manager = RoomManager::new(RoomManagerLimits {
        max_clients: 1,
        ..RoomManagerLimits::default()
    });
    let (alice_tx, _) = crate::broadcast::channel();
    manager
        .try_connect_at("Alice".to_owned(), None, alice_tx, 1_000)
        .unwrap();

    let (bob_tx, _) = crate::broadcast::channel();
    let err = manager
        .try_connect_at("Bob".to_owned(), None, bob_tx, 1_001)
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::ClientLimitReached));
}

#[test]
fn abandoned_session_cleanup_frees_client_capacity() {
    let mut manager = RoomManager::new(RoomManagerLimits {
        max_clients: 1,
        abandoned_session_ttl_ms: 30_000,
        ..RoomManagerLimits::default()
    });
    let (alice_tx, _) = crate::broadcast::channel();
    let alice = manager
        .try_connect_at("Alice".to_owned(), None, alice_tx, 1_000)
        .unwrap();
    manager.disconnect(&alice.player_id, alice.connection_id, 2_000);

    let (bob_tx, _) = crate::broadcast::channel();
    let bob = manager
        .try_connect_at("Bob".to_owned(), None, bob_tx, 32_000)
        .unwrap();

    assert_ne!(bob.player_id, alice.player_id);
    assert_eq!(manager.session_registry.player_name(&alice.player_id), None);
    assert_eq!(
        manager.session_registry.player_name(&bob.player_id),
        Some("Bob")
    );
}

#[test]
fn disconnected_in_room_session_is_protected_from_client_cleanup() {
    let mut manager = RoomManager::new(RoomManagerLimits {
        max_clients: 1,
        abandoned_session_ttl_ms: 30_000,
        ..RoomManagerLimits::default()
    });
    let (alice_tx, _) = crate::broadcast::channel();
    let alice = manager
        .try_connect_at("Alice".to_owned(), None, alice_tx, 1_000)
        .unwrap();
    manager
        .create_room(&alice.player_id, "room".to_owned(), None)
        .unwrap();
    manager.disconnect(&alice.player_id, alice.connection_id, 2_000);

    let (bob_tx, _) = crate::broadcast::channel();
    let err = manager
        .try_connect_at("Bob".to_owned(), None, bob_tx, 32_000)
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::ClientLimitReached));
    assert_eq!(
        manager.session_registry.player_name(&alice.player_id),
        Some("Alice")
    );
}
#[test]
fn saturated_outbound_queue_drops_active_socket() {
    let mut manager = RoomManager::default();
    let (tx, _rx) = crate::broadcast::channel_with_capacity(1);
    let alice = manager.connect("Alice".to_owned(), None, tx);

    manager.respond(&alice.player_id, 1, ServerResult::Ok);
    manager.respond(&alice.player_id, 2, ServerResult::Ok);

    assert_eq!(manager.session_registry.active_count(), 0);
    assert_eq!(
        manager.session_registry.player_name(&alice.player_id),
        Some("Alice")
    );
}
