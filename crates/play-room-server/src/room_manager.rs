use crate::broadcast::{send, OutboundTx};
use crate::identity::{new_player_id, new_room_id, new_session_token};
use play_room_core::{
    CoreError, GameRoom, GameRules, Player, PlayerId, PlayerRole, RoomCommand, RoomEvent, RoomId,
    RoomSnapshot, RoomSummary, SessionToken,
};
use play_room_protocol::{ErrorCode, ServerEvent, ServerMessage, ServerResult, PROTOCOL_VERSION};
use std::collections::BTreeMap;

pub const PARTICIPANT_SEAT_GRACE_MS: u64 = 90_000;
pub const SPECTATOR_NAME_GRACE_MS: u64 = 90_000;

#[derive(Default)]
pub struct RoomManager {
    rooms: BTreeMap<RoomId, GameRoom>,
    sessions: BTreeMap<PlayerId, OutboundTx>,
    player_names: BTreeMap<PlayerId, String>,
    player_rooms: BTreeMap<PlayerId, RoomId>,
    tokens: BTreeMap<SessionToken, PlayerId>,
    seat_expirations: BTreeMap<PlayerId, SeatExpiry>,
    spectator_expirations: BTreeMap<PlayerId, SpectatorExpiry>,
}

#[derive(Clone, Debug)]
pub struct ConnectedPlayer {
    pub player_id: PlayerId,
    pub reconnect_token: SessionToken,
    pub messages: OutboundMessages,
}

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
    pub messages: OutboundMessages,
    pub spectator_expiry: Option<SpectatorExpiry>,
}

#[derive(Clone, Debug)]
pub struct DisconnectOutcome {
    pub messages: OutboundMessages,
    pub seat_expiry: Option<SeatExpiry>,
    pub spectator_expiry: Option<SpectatorExpiry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoomManagerError {
    message: String,
    code: Option<ErrorCode>,
    suggestions: Vec<String>,
}

impl RoomManagerError {
    fn plain(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            suggestions: Vec::new(),
        }
    }

    fn coded(message: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
            suggestions: Vec::new(),
        }
    }

    fn with_suggestions(
        message: impl Into<String>,
        code: ErrorCode,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
            suggestions,
        }
    }

    fn room_not_found(room: impl std::fmt::Display) -> Self {
        Self::coded(format!("room not found: {room}"), ErrorCode::RoomNotFound)
    }

    fn not_in_room() -> Self {
        Self::coded("player is not in a room", ErrorCode::NotInRoom)
    }

    fn duplicate_player_name(name: impl Into<String>, connected: Option<bool>) -> Self {
        let name = name.into();
        let message = match connected {
            Some(false) => format!(
                "{name} is already in this room but currently disconnected. Reconnect with the session token or choose another name."
            ),
            Some(true) => format!("{name} is already in this room. Choose another name."),
            None => format!("player name already exists in this room: {name}"),
        };
        Self::coded(message, ErrorCode::PlayerNameExists)
    }

    fn from_core(error: CoreError) -> Self {
        match error {
            CoreError::RoomNotFound(room_id) => Self::room_not_found(room_id),
            CoreError::RoomFull => Self::coded("room is full", ErrorCode::RoomFull),
            CoreError::DuplicatePlayerName(name) => Self::duplicate_player_name(name, None),
            CoreError::MatchNotFinished => {
                Self::coded("match is not finished", ErrorCode::MatchNotFinished)
            }
            CoreError::HostOnly => Self::coded(
                "only the room host can start the next match",
                ErrorCode::HostOnly,
            ),
            CoreError::AlreadyInRoom
            | CoreError::RoomFinished
            | CoreError::SpectatorsNotAllowed
            | CoreError::SpectatorAction
            | CoreError::PlayerDisconnected
            | CoreError::RoundNotActive
            | CoreError::RoundAlreadyActive
            | CoreError::InvalidMove { .. }
            | CoreError::NotEnoughReadyParticipants
            | CoreError::StaleTimeout
            | CoreError::EmptyName
            | CoreError::InvalidRules(_)
            | CoreError::PlayerNotFound(_) => {
                Self::coded(error.to_string(), ErrorCode::InvalidAction)
            }
        }
    }

    #[cfg(test)]
    fn message(&self) -> &str {
        &self.message
    }

    #[cfg(test)]
    fn code(&self) -> Option<&ErrorCode> {
        self.code.as_ref()
    }

    #[cfg(test)]
    fn suggestions(&self) -> &[String] {
        &self.suggestions
    }

    pub fn into_server_result(self) -> ServerResult {
        ServerResult::Error {
            message: self.message,
            code: self.code,
            suggestions: self.suggestions,
        }
    }
}

type RoundTimer = (u32, u64);
pub type OutboundMessage = (PlayerId, ServerMessage);
pub type OutboundMessages = Vec<OutboundMessage>;
type AppliedRoomCommand = (RoomId, Vec<RoomEvent>, Option<RoundTimer>);

impl RoomManager {
    pub fn connect(
        &mut self,
        name: String,
        token: Option<SessionToken>,
        tx: OutboundTx,
    ) -> ConnectedPlayer {
        if let Some(token) = token {
            if let Some(player_id) = self.tokens.get(&token).cloned() {
                self.sessions.insert(player_id.clone(), tx);
                self.player_names.entry(player_id.clone()).or_insert(name);
                self.seat_expirations.remove(&player_id);
                self.spectator_expirations.remove(&player_id);
                let messages = if let Some(room_id) = self.player_rooms.get(&player_id).cloned() {
                    let events = if let Some(room) = self.rooms.get_mut(&room_id) {
                        room.apply(RoomCommand::Reconnect {
                            player_id: player_id.clone(),
                        })
                        .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    self.room_messages(&room_id, events)
                } else {
                    Vec::new()
                };
                return ConnectedPlayer {
                    player_id,
                    reconnect_token: token,
                    messages,
                };
            }
        }

        let player_id = new_player_id();
        let reconnect_token = new_session_token();
        self.sessions.insert(player_id.clone(), tx);
        self.player_names.insert(player_id.clone(), name);
        self.tokens
            .insert(reconnect_token.clone(), player_id.clone());
        ConnectedPlayer {
            player_id,
            reconnect_token,
            messages: Vec::new(),
        }
    }

    pub fn disconnect(&mut self, player_id: &PlayerId, now_ms: u64) -> DisconnectOutcome {
        self.sessions.remove(player_id);
        if let Some(room_id) = self.player_rooms.get(player_id).cloned() {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                if let Ok(events) = room.apply(RoomCommand::Disconnect {
                    player_id: player_id.clone(),
                }) {
                    let seat_expiry =
                        self.schedule_seat_expiry_if_needed(&room_id, player_id, now_ms);
                    let spectator_expiry =
                        self.schedule_spectator_expiry_if_needed(&room_id, player_id, now_ms);
                    let messages = self.room_messages(&room_id, events);
                    return DisconnectOutcome {
                        messages,
                        seat_expiry,
                        spectator_expiry,
                    };
                }
            }
        }
        self.seat_expirations.remove(player_id);
        self.spectator_expirations.remove(player_id);
        DisconnectOutcome {
            messages: Vec::new(),
            seat_expiry: None,
            spectator_expiry: None,
        }
    }

    pub fn send_to(&self, player_id: &PlayerId, message: ServerMessage) {
        if let Some(tx) = self.sessions.get(player_id) {
            send(tx, message);
        }
    }

    pub fn respond(&self, player_id: &PlayerId, request_id: u64, result: ServerResult) {
        self.send_to(player_id, ServerMessage::Response { request_id, result });
    }

    pub fn welcome(&self, player_id: &PlayerId, request_id: u64, token: SessionToken) {
        self.respond(
            player_id,
            request_id,
            ServerResult::Welcome {
                player_id: player_id.clone(),
                reconnect_token: token,
                protocol_version: PROTOCOL_VERSION,
            },
        );
    }

    pub fn list_rooms(&self) -> Vec<RoomSummary> {
        self.rooms.values().map(GameRoom::summary).collect()
    }

    pub fn create_room(
        &mut self,
        owner_id: &PlayerId,
        name: String,
        rules: Option<GameRules>,
    ) -> Result<(RoomId, OutboundMessages), RoomManagerError> {
        let room_name = name.trim();
        if room_name.is_empty() {
            return Err(RoomManagerError::plain("room name is empty"));
        }
        if self.room_name_exists(room_name) {
            return Err(RoomManagerError::with_suggestions(
                format!("room name already exists: {room_name}"),
                ErrorCode::RoomNameExists,
                self.suggest_room_names(room_name, owner_id),
            ));
        }

        let player_name = self
            .player_names
            .get(owner_id)
            .cloned()
            .unwrap_or_else(|| owner_id.as_str().to_owned());
        let rules = rules.unwrap_or_default();
        rules.validate().map_err(RoomManagerError::from_core)?;
        let room_id = new_room_id();
        let host = Player::participant(owner_id.clone(), player_name);
        let room = GameRoom::new(room_id.clone(), room_name.to_owned(), rules, host)
            .map_err(RoomManagerError::from_core)?;

        let mut messages = OutboundMessages::new();
        if let Some(previous) = self.player_rooms.get(owner_id).cloned() {
            messages.extend(self.leave_room(owner_id, &previous)?);
        }

        self.player_rooms.insert(owner_id.clone(), room_id.clone());
        self.rooms.insert(room_id.clone(), room);
        messages.extend(self.room_messages(&room_id, Vec::new()));
        Ok((room_id, messages))
    }

    pub fn join_room(
        &mut self,
        player_id: &PlayerId,
        room_id_or_name: &RoomId,
    ) -> Result<OutboundMessages, RoomManagerError> {
        self.join_room_as(player_id, room_id_or_name, PlayerRole::Participant)
    }

    pub fn spectate_room(
        &mut self,
        player_id: &PlayerId,
        room_id_or_name: &RoomId,
    ) -> Result<OutboundMessages, RoomManagerError> {
        self.join_room_as(player_id, room_id_or_name, PlayerRole::Spectator)
    }

    fn join_room_as(
        &mut self,
        player_id: &PlayerId,
        room_id_or_name: &RoomId,
        role: PlayerRole,
    ) -> Result<OutboundMessages, RoomManagerError> {
        let room_id = self.resolve_room_id(room_id_or_name)?;
        let player_name = self
            .player_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| player_id.as_str().to_owned());
        let player = match role {
            PlayerRole::Participant => Player::participant(player_id.clone(), player_name.clone()),
            PlayerRole::Spectator => Player::spectator(player_id.clone(), player_name.clone()),
        };

        let room = self
            .rooms
            .get(&room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(&room_id))?;
        if let Some(existing) = room.player_named(&player_name) {
            if existing.id != *player_id {
                return Err(RoomManagerError::duplicate_player_name(
                    existing.name,
                    Some(existing.connected),
                ));
            }
        }

        let mut probe = room.clone();
        probe
            .apply(RoomCommand::Join {
                player: player.clone(),
            })
            .map_err(RoomManagerError::from_core)?;

        let mut messages = OutboundMessages::new();
        if let Some(previous) = self.player_rooms.get(player_id).cloned() {
            if previous != room_id {
                messages.extend(self.leave_room(player_id, &previous)?);
            }
        }

        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(&room_id))?;
        let events = room
            .apply(RoomCommand::Join { player })
            .map_err(RoomManagerError::from_core)?;
        self.player_rooms.insert(player_id.clone(), room_id.clone());
        messages.extend(self.room_messages(&room_id, events));
        Ok(messages)
    }

    fn resolve_room_id(&self, room_id_or_name: &RoomId) -> Result<RoomId, RoomManagerError> {
        if self.rooms.contains_key(room_id_or_name) {
            return Ok(room_id_or_name.clone());
        }

        let requested = room_id_or_name.as_str().trim();
        let matches: Vec<RoomId> = self
            .rooms
            .iter()
            .filter(|(_, room)| room.name().trim().eq_ignore_ascii_case(requested))
            .map(|(room_id, _)| room_id.clone())
            .collect();

        match matches.as_slice() {
            [room_id] => Ok(room_id.clone()),
            [] => Err(RoomManagerError::room_not_found(requested)),
            _ => Err(RoomManagerError::coded(
                format!("multiple rooms named {requested}; use the room id from /rooms"),
                ErrorCode::InvalidRequest,
            )),
        }
    }

    pub fn leave_current_room(
        &mut self,
        player_id: &PlayerId,
    ) -> Result<OutboundMessages, RoomManagerError> {
        let room_id = self
            .player_rooms
            .get(player_id)
            .cloned()
            .ok_or_else(RoomManagerError::not_in_room)?;
        self.leave_room(player_id, &room_id)
    }

    fn leave_room(
        &mut self,
        player_id: &PlayerId,
        room_id: &RoomId,
    ) -> Result<OutboundMessages, RoomManagerError> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(room_id))?;
        let events = room
            .apply(RoomCommand::Leave {
                player_id: player_id.clone(),
            })
            .map_err(RoomManagerError::from_core)?;
        self.seat_expirations.remove(player_id);
        self.spectator_expirations.remove(player_id);
        self.player_rooms.remove(player_id);
        let messages = self.room_messages(room_id, events);
        let remove_room = self
            .rooms
            .get(room_id)
            .map(|r| r.player_ids().is_empty())
            .unwrap_or(false);
        if remove_room {
            self.rooms.remove(room_id);
        }
        Ok(messages)
    }

    pub fn apply_to_current_room(
        &mut self,
        player_id: &PlayerId,
        command: RoomCommand,
    ) -> Result<AppliedRoomCommand, RoomManagerError> {
        let room_id = self
            .player_rooms
            .get(player_id)
            .cloned()
            .ok_or_else(RoomManagerError::not_in_room)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(&room_id))?;
        let events = room.apply(command).map_err(RoomManagerError::from_core)?;
        let timer = events.iter().find_map(|event| {
            if let RoomEvent::RoundStarted { round, deadline_ms } = event {
                Some((*round, *deadline_ms))
            } else {
                None
            }
        });
        Ok((room_id, events, timer))
    }

    pub fn timeout_room(
        &mut self,
        room_id: &RoomId,
        round: u32,
        now_ms: u64,
    ) -> Result<OutboundMessages, RoomManagerError> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(room_id))?;
        let events = room
            .apply(RoomCommand::TimeoutRound { round, now_ms })
            .map_err(RoomManagerError::from_core)?;
        Ok(self.room_messages(room_id, events))
    }

    pub fn expire_participant_seat(
        &mut self,
        expiry: &SeatExpiry,
    ) -> Result<ExpiryOutcome, RoomManagerError> {
        let Some(current) = self.seat_expirations.get(&expiry.player_id) else {
            return Ok(ExpiryOutcome {
                messages: Vec::new(),
                spectator_expiry: None,
            });
        };
        if current != expiry {
            return Ok(ExpiryOutcome {
                messages: Vec::new(),
                spectator_expiry: None,
            });
        }
        self.seat_expirations.remove(&expiry.player_id);

        let Some(room) = self.rooms.get_mut(&expiry.room_id) else {
            return Ok(ExpiryOutcome {
                messages: Vec::new(),
                spectator_expiry: None,
            });
        };
        let events = room
            .apply(RoomCommand::ExpireParticipantSeat {
                player_id: expiry.player_id.clone(),
            })
            .map_err(RoomManagerError::from_core)?;
        if events.is_empty() {
            return Ok(ExpiryOutcome {
                messages: Vec::new(),
                spectator_expiry: None,
            });
        }

        let spectator_expiry = self.schedule_spectator_expiry_if_needed(
            &expiry.room_id,
            &expiry.player_id,
            expiry.expires_at_ms,
        );
        let messages = self.room_messages(&expiry.room_id, events);
        Ok(ExpiryOutcome {
            messages,
            spectator_expiry,
        })
    }

    pub fn expire_spectator(
        &mut self,
        expiry: &SpectatorExpiry,
    ) -> Result<OutboundMessages, RoomManagerError> {
        let Some(current) = self.spectator_expirations.get(&expiry.player_id) else {
            return Ok(Vec::new());
        };
        if current != expiry {
            return Ok(Vec::new());
        }
        self.spectator_expirations.remove(&expiry.player_id);

        let Some(room) = self.rooms.get(&expiry.room_id) else {
            self.player_rooms.remove(&expiry.player_id);
            return Ok(Vec::new());
        };
        let should_leave = room.snapshot().players.into_iter().any(|player| {
            player.id == expiry.player_id
                && player.role == PlayerRole::Spectator
                && !player.connected
        });
        if !should_leave {
            return Ok(Vec::new());
        }

        self.leave_room(&expiry.player_id, &expiry.room_id)
    }

    fn schedule_seat_expiry_if_needed(
        &mut self,
        room_id: &RoomId,
        player_id: &PlayerId,
        now_ms: u64,
    ) -> Option<SeatExpiry> {
        let should_expire = self
            .rooms
            .get(room_id)
            .and_then(|room| {
                room.snapshot()
                    .players
                    .into_iter()
                    .find(|player| &player.id == player_id)
            })
            .map(|player| player.role == PlayerRole::Participant && !player.connected)
            .unwrap_or(false);

        if !should_expire {
            self.seat_expirations.remove(player_id);
            return None;
        }

        let expiry = SeatExpiry {
            room_id: room_id.clone(),
            player_id: player_id.clone(),
            expires_at_ms: now_ms.saturating_add(PARTICIPANT_SEAT_GRACE_MS),
        };
        self.seat_expirations
            .insert(player_id.clone(), expiry.clone());
        Some(expiry)
    }

    fn schedule_spectator_expiry_if_needed(
        &mut self,
        room_id: &RoomId,
        player_id: &PlayerId,
        now_ms: u64,
    ) -> Option<SpectatorExpiry> {
        let should_expire = self
            .rooms
            .get(room_id)
            .and_then(|room| {
                room.snapshot()
                    .players
                    .into_iter()
                    .find(|player| &player.id == player_id)
            })
            .map(|player| player.role == PlayerRole::Spectator && !player.connected)
            .unwrap_or(false);

        if !should_expire {
            self.spectator_expirations.remove(player_id);
            return None;
        }

        let expiry = SpectatorExpiry {
            room_id: room_id.clone(),
            player_id: player_id.clone(),
            expires_at_ms: now_ms.saturating_add(SPECTATOR_NAME_GRACE_MS),
        };
        self.spectator_expirations
            .insert(player_id.clone(), expiry.clone());
        Some(expiry)
    }
    pub fn room_messages(&self, room_id: &RoomId, events: Vec<RoomEvent>) -> OutboundMessages {
        let Some(room) = self.rooms.get(room_id) else {
            return Vec::new();
        };
        let recipients = room.player_ids();
        let Some(room_snapshot) = self.room_snapshot(room_id) else {
            return Vec::new();
        };

        let mut messages = Vec::new();
        for event in events {
            let msg = ServerMessage::Event {
                event: ServerEvent::RoomEvent {
                    room_id: room_id.clone(),
                    event,
                },
            };
            for recipient in &recipients {
                messages.push((recipient.clone(), msg.clone()));
            }
        }
        let snapshot = ServerMessage::Event {
            event: ServerEvent::RoomSnapshot {
                room: room_snapshot,
            },
        };
        for recipient in recipients {
            messages.push((recipient, snapshot.clone()));
        }
        messages
    }

    fn room_snapshot(&self, room_id: &RoomId) -> Option<RoomSnapshot> {
        let mut snapshot = self.rooms.get(room_id)?.snapshot();
        for player in &mut snapshot.players {
            player.participant_seat_expires_at_ms = self
                .seat_expirations
                .get(&player.id)
                .filter(|expiry| &expiry.room_id == room_id)
                .map(|expiry| expiry.expires_at_ms);
            player.spectator_expires_at_ms = self
                .spectator_expirations
                .get(&player.id)
                .filter(|expiry| &expiry.room_id == room_id)
                .map(|expiry| expiry.expires_at_ms);
        }
        Some(snapshot)
    }

    pub fn flush_messages(&self, messages: OutboundMessages) {
        for (player_id, message) in messages {
            self.send_to(&player_id, message);
        }
    }

    fn room_name_exists(&self, name: &str) -> bool {
        self.rooms
            .values()
            .any(|room| room.name().trim().eq_ignore_ascii_case(name.trim()))
    }

    fn suggest_room_names(&self, desired: &str, owner_id: &PlayerId) -> Vec<String> {
        let base = slugify(desired);
        let mut suggestions = Vec::new();
        push_available_room_name(&mut suggestions, self, format!("{base}-2"));

        if let Some(owner_slug) = self
            .player_names
            .get(owner_id)
            .map(|name| slugify(name))
            .filter(|slug| !slug.is_empty() && slug != &base)
        {
            push_available_room_name(&mut suggestions, self, format!("{base}-{owner_slug}"));
        }

        let mut suffix = 3;
        while suggestions.len() < 3 {
            push_available_room_name(&mut suggestions, self, format!("{base}-{suffix}"));
            suffix += 1;
        }
        suggestions
    }
}

fn push_available_room_name(names: &mut Vec<String>, manager: &RoomManager, candidate: String) {
    if !names
        .iter()
        .any(|name| name.eq_ignore_ascii_case(&candidate))
        && !manager.room_name_exists(&candidate)
    {
        names.push(candidate);
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.trim().chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "room".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(manager.player_rooms.get(&guest_id), Some(&room_id));
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
    fn duplicate_disconnected_player_name_is_rejected_with_clear_message() {
        let mut manager = RoomManager::default();
        let (alice_tx, _) = crate::broadcast::channel();
        let alice = manager.connect("Alice".to_owned(), None, alice_tx);
        let (room_id, _) = manager
            .create_room(&alice.player_id, "testroom".to_owned(), None)
            .unwrap();
        manager.disconnect(&alice.player_id, 1_000);

        let (other_tx, _) = crate::broadcast::channel();
        let other = manager.connect("alice".to_owned(), None, other_tx);
        let err = manager.join_room(&other.player_id, &room_id).unwrap_err();

        assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
        assert!(err.message().contains("currently disconnected"));
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

        assert_eq!(manager.player_rooms.get(&alice), Some(&new_room_id));
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

        assert_eq!(manager.player_rooms.get(&alice), Some(&new_room_id));
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

        assert_eq!(
            manager.player_rooms.get(&alice.player_id),
            Some(&old_room_id)
        );
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

        manager.spectate_room(&mira.player_id, &room_id).unwrap();
        let mira_clone = connect_named(&mut manager, "mira");
        let spectator_conflict = manager
            .spectate_room(&mira_clone.player_id, &room_id)
            .unwrap_err();

        assert_eq!(
            spectator_conflict.code(),
            Some(&ErrorCode::PlayerNameExists)
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

        let outcome = manager.disconnect(&mira.player_id, 5_000);
        let expiry = outcome.spectator_expiry.unwrap();
        let snapshot = manager.room_snapshot(&room_id).unwrap();

        assert!(outcome.seat_expiry.is_none());
        assert_eq!(expiry.expires_at_ms, 95_000);
        assert!(snapshot.players.iter().any(|player| {
            player.id == mira.player_id && player.spectator_expires_at_ms == Some(95_000)
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
            .disconnect(&mira.player_id, 1_000)
            .spectator_expiry
            .unwrap();

        let err = manager
            .spectate_room(&other_mira.player_id, &room_id)
            .unwrap_err();
        assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));

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
            .disconnect(&mira.player_id, 1_000)
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

        let outcome = manager.disconnect(&alice.player_id, 1_000);
        let err = manager.join_room(&carol.player_id, &room_id).unwrap_err();

        assert_eq!(err.code(), Some(&ErrorCode::RoomFull));
        assert_eq!(
            outcome
                .seat_expiry
                .as_ref()
                .map(|expiry| expiry.expires_at_ms),
            Some(91_000)
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

        let outcome = manager.disconnect(&alice.player_id, 1_000);
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
            .disconnect(&alice.player_id, 1_000)
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
            .disconnect(&alice.player_id, 1_000)
            .seat_expiry
            .unwrap();
        let outcome = manager.expire_participant_seat(&expiry).unwrap();
        let spectator_expiry = outcome.spectator_expiry.unwrap();

        let err = manager
            .spectate_room(&other_alice.player_id, &room_id)
            .unwrap_err();
        assert_eq!(err.code(), Some(&ErrorCode::PlayerNameExists));
        assert!(err.message().contains("currently disconnected"));

        manager.expire_spectator(&spectator_expiry).unwrap();
        let messages = manager
            .spectate_room(&other_alice.player_id, &room_id)
            .unwrap();

        assert_eq!(spectator_expiry.expires_at_ms, 181_000);
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
            .disconnect(&alice.player_id, 1_000)
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
            .disconnect(&alice.player_id, 1_000)
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
            .disconnect(&alice.player_id, 1_000)
            .seat_expiry
            .unwrap();
        manager.leave_current_room(&alice.player_id).unwrap();

        let outcome = manager.expire_participant_seat(&expiry).unwrap();

        assert!(outcome.messages.is_empty());
        assert!(outcome.spectator_expiry.is_none());
        assert!(!room_has_player(&manager, &room_id, &alice.player_id));
    }

    #[test]
    fn reconnect_returns_room_snapshot_to_reconnecting_player() {
        let mut manager = RoomManager::default();
        let (tx, _) = crate::broadcast::channel();
        let connected = manager.connect("alice".to_owned(), None, tx);
        let (room_id, _) = manager
            .create_room(&connected.player_id, "testroom".to_owned(), None)
            .unwrap();
        manager.disconnect(&connected.player_id, 1_000);

        let (reconnect_tx, _) = crate::broadcast::channel();
        let reconnected = manager.connect(
            String::new(),
            Some(connected.reconnect_token.clone()),
            reconnect_tx,
        );

        assert_eq!(reconnected.player_id, connected.player_id);
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
        assert_eq!(manager.player_rooms.get(&alice), Some(&old_room_id));
        assert!(room_has_player(&manager, &old_room_id, &alice));
        assert!(!room_has_player(&manager, &full_room_id, &alice));
    }
}
