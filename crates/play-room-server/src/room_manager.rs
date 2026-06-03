use crate::broadcast::OutboundTx;
use crate::expiry as expiry_policy;
pub use crate::expiry::{DisconnectOutcome, ExpiryOutcome, SeatExpiry, SpectatorExpiry};
use crate::fanout::{self, OutboundMessages};
use crate::membership::RoomMemberships;
use crate::room_lifecycle::{self, RoundTimer};
use crate::room_registry::{RoomLookupError, RoomRegistry};
use crate::session_registry::SessionRegistry;
use play_room_core::{
    CoreError, GameRules, PlayerId, PlayerRole, RoomCommand, RoomEvent, RoomId, RoomSnapshot,
    RoomSummary, SessionToken,
};
use play_room_protocol::{ErrorCode, ServerEvent, ServerMessage, ServerResult, PROTOCOL_VERSION};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct RoomManager {
    rooms: RoomRegistry,
    session_registry: SessionRegistry,
    room_memberships: RoomMemberships,
    seat_expirations: BTreeMap<PlayerId, SeatExpiry>,
    spectator_expirations: BTreeMap<PlayerId, SpectatorExpiry>,
}

#[derive(Clone, Debug)]
pub struct ConnectedPlayer {
    pub player_id: PlayerId,
    pub reconnect_token: SessionToken,
    pub messages: OutboundMessages,
    pub reconnected: bool,
    pub stale_token_replaced: bool,
    pub room_restored: bool,
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
    fn from_room_lookup(error: RoomLookupError) -> Self {
        match error {
            RoomLookupError::NotFound(requested) => Self::room_not_found(requested),
            RoomLookupError::Ambiguous(requested) => Self::coded(
                format!("multiple rooms named {requested}; use the room id from /rooms"),
                ErrorCode::InvalidRequest,
            ),
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

type AppliedRoomCommand = (RoomId, Vec<RoomEvent>, Option<RoundTimer>);

impl RoomManager {
    pub fn connect(
        &mut self,
        name: String,
        token: Option<SessionToken>,
        tx: OutboundTx,
    ) -> ConnectedPlayer {
        let registered = self.session_registry.connect(name, token, tx);
        let player_id = registered.player_id.clone();
        let reconnect_token = registered.reconnect_token.clone();
        let (messages, room_restored) = if registered.reconnected {
            self.seat_expirations.remove(&player_id);
            self.spectator_expirations.remove(&player_id);
            if let Some(room_id) = self.room_memberships.room_for(&player_id).cloned() {
                let events = if let Some(room) = self.rooms.get_mut(&room_id) {
                    room.apply(RoomCommand::Reconnect {
                        player_id: player_id.clone(),
                    })
                    .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let messages = self.room_messages(&room_id, events);
                let room_restored = messages.iter().any(|(target, message)| {
                    target == &player_id
                        && matches!(
                            message,
                            ServerMessage::Event {
                                event: ServerEvent::RoomSnapshot { .. }
                            }
                        )
                });
                (messages, room_restored)
            } else {
                (Vec::new(), false)
            }
        } else if registered.reconnect_token_replaced {
            (
                vec![(
                    player_id.clone(),
                    ServerMessage::Event {
                        event: ServerEvent::Notice {
                            message: "Reconnect token was not recognized; started a new session."
                                .to_owned(),
                        },
                    },
                )],
                false,
            )
        } else {
            (Vec::new(), false)
        };

        ConnectedPlayer {
            player_id,
            reconnect_token,
            messages,
            reconnected: registered.reconnected,
            stale_token_replaced: registered.reconnect_token_replaced,
            room_restored,
        }
    }
    pub fn disconnect(&mut self, player_id: &PlayerId, now_ms: u64) -> DisconnectOutcome {
        self.session_registry.disconnect_socket(player_id);
        if let Some(room_id) = self.room_memberships.room_for(player_id).cloned() {
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
        fanout::send_to(self.session_registry.sessions(), player_id, message);
    }
    pub fn respond(&self, player_id: &PlayerId, request_id: u64, result: ServerResult) {
        self.send_to(player_id, ServerMessage::Response { request_id, result });
    }

    pub fn welcome(&self, connected: &ConnectedPlayer, request_id: u64) {
        self.respond(
            &connected.player_id,
            request_id,
            ServerResult::Welcome {
                player_id: connected.player_id.clone(),
                reconnect_token: connected.reconnect_token.clone(),
                protocol_version: PROTOCOL_VERSION,
                reconnected: connected.reconnected,
                stale_token_replaced: connected.stale_token_replaced,
                room_restored: connected.room_restored,
            },
        );
    }

    pub fn list_rooms(&self) -> Vec<RoomSummary> {
        self.rooms.summaries()
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
        if self.rooms.room_name_exists(room_name) {
            return Err(RoomManagerError::with_suggestions(
                format!("room name already exists: {room_name}"),
                ErrorCode::RoomNameExists,
                self.rooms
                    .suggest_room_names(room_name, self.session_registry.player_name(owner_id)),
            ));
        }

        let player_name = self.session_registry.player_name_or_id(owner_id);
        let rules = rules.unwrap_or_default();
        rules.validate().map_err(RoomManagerError::from_core)?;
        let (room_id, room) = room_lifecycle::create_room(owner_id, room_name, rules, player_name)
            .map_err(RoomManagerError::from_core)?;

        let mut messages = OutboundMessages::new();
        if let Some(previous) = self.room_memberships.room_for(owner_id).cloned() {
            messages.extend(self.leave_room(owner_id, &previous)?);
        }

        self.room_memberships
            .set_room(owner_id.clone(), room_id.clone());
        self.rooms.insert(room);
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
        let room_id = self
            .rooms
            .resolve_room_id(room_id_or_name)
            .map_err(RoomManagerError::from_room_lookup)?;
        let player_name = self.session_registry.player_name_or_id(player_id);
        let player = room_lifecycle::player_for_role(player_id, player_name.clone(), role);

        let room = self
            .rooms
            .get(&room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(&room_id))?;
        if let Some(conflict) = room_lifecycle::player_name_conflict(room, player_id, &player_name)
        {
            return Err(RoomManagerError::duplicate_player_name(
                conflict.name,
                Some(conflict.connected),
            ));
        }

        let mut probe = room.clone();
        probe
            .apply(RoomCommand::Join {
                player: player.clone(),
            })
            .map_err(RoomManagerError::from_core)?;

        let mut messages = OutboundMessages::new();
        if let Some(previous) = self.room_memberships.room_for(player_id).cloned() {
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
        self.room_memberships
            .set_room(player_id.clone(), room_id.clone());
        messages.extend(self.room_messages(&room_id, events));
        Ok(messages)
    }

    pub fn leave_current_room(
        &mut self,
        player_id: &PlayerId,
    ) -> Result<OutboundMessages, RoomManagerError> {
        let room_id = self
            .room_memberships
            .room_for(player_id)
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
        self.room_memberships.remove(player_id);
        let messages = self.room_messages(room_id, events);
        let remove_room = self.rooms.should_remove_room(room_id);
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
            .room_memberships
            .room_for(player_id)
            .cloned()
            .ok_or_else(RoomManagerError::not_in_room)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| RoomManagerError::room_not_found(&room_id))?;
        let events = room.apply(command).map_err(RoomManagerError::from_core)?;
        let timer = room_lifecycle::round_timer(&events);
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
            self.room_memberships.remove(&expiry.player_id);
            return Ok(Vec::new());
        };
        let should_leave =
            expiry_policy::disconnected_spectator_should_leave(room, &expiry.player_id);
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
        let Some(expiry) = expiry_policy::participant_seat_expiry(
            self.rooms.get(room_id),
            room_id,
            player_id,
            now_ms,
        ) else {
            self.seat_expirations.remove(player_id);
            return None;
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
        let Some(expiry) = expiry_policy::spectator_name_expiry(
            self.rooms.get(room_id),
            room_id,
            player_id,
            now_ms,
        ) else {
            self.spectator_expirations.remove(player_id);
            return None;
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
        fanout::room_messages(room_id, room_snapshot, events, recipients)
    }

    fn room_snapshot(&self, room_id: &RoomId) -> Option<RoomSnapshot> {
        let mut snapshot = self.rooms.get(room_id)?.snapshot();
        expiry_policy::annotate_snapshot_expirations(
            &mut snapshot,
            room_id,
            &self.seat_expirations,
            &self.spectator_expirations,
        );
        Some(snapshot)
    }

    pub fn flush_messages(&self, messages: OutboundMessages) {
        fanout::flush_messages(self.session_registry.sessions(), messages);
    }
    #[cfg(test)]
    fn player_room(&self, player_id: &PlayerId) -> Option<&RoomId> {
        self.room_memberships.room_for(player_id)
    }
}

#[cfg(test)]
mod tests;
