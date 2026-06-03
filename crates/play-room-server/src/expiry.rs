use play_room_core::{GameRoom, PlayerId, PlayerRole, RoomId, RoomSnapshot};
use std::collections::BTreeMap;

pub const PARTICIPANT_SEAT_GRACE_MS: u64 = 90_000;
pub const SPECTATOR_NAME_GRACE_MS: u64 = 90_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeatExpiry {
    pub room_id: RoomId,
    pub player_id: PlayerId,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpectatorExpiry {
    pub room_id: RoomId,
    pub player_id: PlayerId,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ExpiryOutcome {
    pub messages: crate::fanout::OutboundMessages,
    pub spectator_expiry: Option<SpectatorExpiry>,
}

#[derive(Clone, Debug)]
pub struct DisconnectOutcome {
    pub messages: crate::fanout::OutboundMessages,
    pub seat_expiry: Option<SeatExpiry>,
    pub spectator_expiry: Option<SpectatorExpiry>,
}

pub fn participant_seat_expiry(
    room: Option<&GameRoom>,
    room_id: &RoomId,
    player_id: &PlayerId,
    now_ms: u64,
) -> Option<SeatExpiry> {
    let should_expire = room
        .and_then(|room| {
            room.snapshot()
                .players
                .into_iter()
                .find(|player| &player.id == player_id)
        })
        .map(|player| player.role == PlayerRole::Participant && !player.connected)
        .unwrap_or(false);

    should_expire.then(|| SeatExpiry {
        room_id: room_id.clone(),
        player_id: player_id.clone(),
        expires_at_ms: now_ms.saturating_add(PARTICIPANT_SEAT_GRACE_MS),
    })
}

pub fn spectator_name_expiry(
    room: Option<&GameRoom>,
    room_id: &RoomId,
    player_id: &PlayerId,
    now_ms: u64,
) -> Option<SpectatorExpiry> {
    let should_expire = room
        .and_then(|room| {
            room.snapshot()
                .players
                .into_iter()
                .find(|player| &player.id == player_id)
        })
        .map(|player| player.role == PlayerRole::Spectator && !player.connected)
        .unwrap_or(false);

    should_expire.then(|| SpectatorExpiry {
        room_id: room_id.clone(),
        player_id: player_id.clone(),
        expires_at_ms: now_ms.saturating_add(SPECTATOR_NAME_GRACE_MS),
    })
}

pub fn disconnected_spectator_should_leave(room: &GameRoom, player_id: &PlayerId) -> bool {
    room.snapshot().players.into_iter().any(|player| {
        player.id == *player_id && player.role == PlayerRole::Spectator && !player.connected
    })
}

pub fn annotate_snapshot_expirations(
    snapshot: &mut RoomSnapshot,
    room_id: &RoomId,
    seat_expirations: &BTreeMap<PlayerId, SeatExpiry>,
    spectator_expirations: &BTreeMap<PlayerId, SpectatorExpiry>,
) {
    for player in &mut snapshot.players {
        player.participant_seat_expires_at_ms = seat_expirations
            .get(&player.id)
            .filter(|expiry| &expiry.room_id == room_id)
            .map(|expiry| expiry.expires_at_ms);
        player.spectator_expires_at_ms = spectator_expirations
            .get(&player.id)
            .filter(|expiry| &expiry.room_id == room_id)
            .map(|expiry| expiry.expires_at_ms);
    }
}
