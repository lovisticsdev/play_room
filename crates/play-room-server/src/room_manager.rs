use crate::broadcast::{send, OutboundTx};
use crate::identity::{new_player_id, new_room_id, new_session_token};
use play_room_core::{
    GameRoom, GameRules, Player, PlayerId, PlayerRole, RoomCommand, RoomEvent, RoomId, RoomSummary,
    SessionToken,
};
use play_room_protocol::{ServerEvent, ServerMessage, ServerResult, PROTOCOL_VERSION};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct RoomManager {
    rooms: BTreeMap<RoomId, GameRoom>,
    sessions: BTreeMap<PlayerId, OutboundTx>,
    player_names: BTreeMap<PlayerId, String>,
    player_rooms: BTreeMap<PlayerId, RoomId>,
    tokens: BTreeMap<SessionToken, PlayerId>,
}

#[derive(Clone, Debug)]
pub struct ConnectedPlayer {
    pub player_id: PlayerId,
    pub reconnect_token: SessionToken,
}

type RoundTimer = (u32, u64);
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
                if let Some(room_id) = self.player_rooms.get(&player_id).cloned() {
                    if let Some(room) = self.rooms.get_mut(&room_id) {
                        let _ = room.apply(RoomCommand::Reconnect {
                            player_id: player_id.clone(),
                        });
                    }
                }
                return ConnectedPlayer {
                    player_id,
                    reconnect_token: token,
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
        }
    }

    pub fn disconnect(&mut self, player_id: &PlayerId) -> Vec<(PlayerId, ServerMessage)> {
        self.sessions.remove(player_id);
        if let Some(room_id) = self.player_rooms.get(player_id).cloned() {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                if let Ok(events) = room.apply(RoomCommand::Disconnect {
                    player_id: player_id.clone(),
                }) {
                    return self.room_messages(&room_id, events);
                }
            }
        }
        Vec::new()
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
    ) -> Result<RoomId, String> {
        let player_name = self
            .player_names
            .get(owner_id)
            .cloned()
            .unwrap_or_else(|| "player".to_owned());
        if let Some(previous) = self.player_rooms.get(owner_id).cloned() {
            self.leave_room(owner_id, &previous).ok();
        }
        let room_id = new_room_id();
        let host = Player::participant(owner_id.clone(), player_name);
        let room = GameRoom::new(room_id.clone(), name, rules.unwrap_or_default(), host)
            .map_err(|e| e.to_string())?;
        self.player_rooms.insert(owner_id.clone(), room_id.clone());
        self.rooms.insert(room_id.clone(), room);
        Ok(room_id)
    }

    pub fn join_room(
        &mut self,
        player_id: &PlayerId,
        room_id_or_name: &RoomId,
    ) -> Result<Vec<(PlayerId, ServerMessage)>, String> {
        self.join_room_as(player_id, room_id_or_name, PlayerRole::Participant)
    }

    pub fn spectate_room(
        &mut self,
        player_id: &PlayerId,
        room_id_or_name: &RoomId,
    ) -> Result<Vec<(PlayerId, ServerMessage)>, String> {
        self.join_room_as(player_id, room_id_or_name, PlayerRole::Spectator)
    }

    fn join_room_as(
        &mut self,
        player_id: &PlayerId,
        room_id_or_name: &RoomId,
        role: PlayerRole,
    ) -> Result<Vec<(PlayerId, ServerMessage)>, String> {
        let room_id = self.resolve_room_id(room_id_or_name)?;
        if let Some(previous) = self.player_rooms.get(player_id).cloned() {
            if previous != room_id {
                self.leave_room(player_id, &previous).ok();
            }
        }
        let player_name = self
            .player_names
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| "player".to_owned());
        let player = match role {
            PlayerRole::Participant => Player::participant(player_id.clone(), player_name),
            PlayerRole::Spectator => Player::spectator(player_id.clone(), player_name),
        };
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| format!("room not found: {room_id}"))?;
        let events = room
            .apply(RoomCommand::Join { player })
            .map_err(|e| e.to_string())?;
        self.player_rooms.insert(player_id.clone(), room_id.clone());
        Ok(self.room_messages(&room_id, events))
    }

    fn resolve_room_id(&self, room_id_or_name: &RoomId) -> Result<RoomId, String> {
        if self.rooms.contains_key(room_id_or_name) {
            return Ok(room_id_or_name.clone());
        }

        let requested = room_id_or_name.as_str();
        let matches: Vec<RoomId> = self
            .rooms
            .iter()
            .filter(|(_, room)| room.name() == requested)
            .map(|(room_id, _)| room_id.clone())
            .collect();

        match matches.as_slice() {
            [room_id] => Ok(room_id.clone()),
            [] => Err(format!("room not found: {requested}")),
            _ => Err(format!(
                "multiple rooms named {requested}; use the room id from /rooms"
            )),
        }
    }

    pub fn leave_current_room(
        &mut self,
        player_id: &PlayerId,
    ) -> Result<Vec<(PlayerId, ServerMessage)>, String> {
        let room_id = self
            .player_rooms
            .get(player_id)
            .cloned()
            .ok_or_else(|| "player is not in a room".to_owned())?;
        self.leave_room(player_id, &room_id)
    }

    fn leave_room(
        &mut self,
        player_id: &PlayerId,
        room_id: &RoomId,
    ) -> Result<Vec<(PlayerId, ServerMessage)>, String> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| format!("room not found: {room_id}"))?;
        let events = room
            .apply(RoomCommand::Leave {
                player_id: player_id.clone(),
            })
            .map_err(|e| e.to_string())?;
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
    ) -> Result<AppliedRoomCommand, String> {
        let room_id = self
            .player_rooms
            .get(player_id)
            .cloned()
            .ok_or_else(|| "player is not in a room".to_owned())?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| format!("room not found: {room_id}"))?;
        let events = room.apply(command).map_err(|e| e.to_string())?;
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
    ) -> Result<Vec<(PlayerId, ServerMessage)>, String> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| format!("room not found: {room_id}"))?;
        let events = room
            .apply(RoomCommand::TimeoutRound { round, now_ms })
            .map_err(|e| e.to_string())?;
        Ok(self.room_messages(room_id, events))
    }

    pub fn room_messages(
        &self,
        room_id: &RoomId,
        events: Vec<RoomEvent>,
    ) -> Vec<(PlayerId, ServerMessage)> {
        let Some(room) = self.rooms.get(room_id) else {
            return Vec::new();
        };
        let recipients = room.player_ids();
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
                room: room.snapshot(),
            },
        };
        for recipient in recipients {
            messages.push((recipient, snapshot.clone()));
        }
        messages
    }

    pub fn flush_messages(&self, messages: Vec<(PlayerId, ServerMessage)>) {
        for (player_id, message) in messages {
            self.send_to(&player_id, message);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_room_by_exact_name_when_id_is_not_used() {
        let mut manager = RoomManager::default();
        let host_id = PlayerId::new("host");
        let room_id = manager
            .create_room(&host_id, "testroom".to_owned(), None)
            .unwrap();
        let guest_id = PlayerId::new("guest");

        let messages = manager
            .join_room(&guest_id, &RoomId::new("testroom"))
            .unwrap();

        assert!(!messages.is_empty());
        assert_eq!(manager.player_rooms.get(&guest_id), Some(&room_id));
    }

    #[test]
    fn duplicate_room_names_require_the_room_id() {
        let mut manager = RoomManager::default();
        manager
            .create_room(&PlayerId::new("host-one"), "testroom".to_owned(), None)
            .unwrap();
        manager
            .create_room(&PlayerId::new("host-two"), "testroom".to_owned(), None)
            .unwrap();

        let err = manager
            .join_room(&PlayerId::new("guest"), &RoomId::new("testroom"))
            .unwrap_err();

        assert!(err.contains("multiple rooms named testroom"));
    }
    #[test]
    fn can_spectate_a_full_room_by_exact_name() {
        let mut manager = RoomManager::default();
        let host_id = PlayerId::new("host");
        let room_id = manager
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
}
