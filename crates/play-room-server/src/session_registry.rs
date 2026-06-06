use crate::broadcast::{self, OutboundTx};
use crate::identity::{new_player_id, new_session_token};
use play_room_core::{CoreError, PlayerId, SessionToken};
use play_room_protocol::{ServerEvent, ServerMessage};
use std::collections::{BTreeMap, BTreeSet};

const FALLBACK_DISPLAY_NAME: &str = "Guest";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ConnectionId(u64);

#[derive(Default)]
pub struct SessionRegistry {
    sessions: BTreeMap<PlayerId, OutboundTx>,
    connection_ids: BTreeMap<PlayerId, ConnectionId>,
    player_names: BTreeMap<PlayerId, String>,
    tokens: BTreeMap<SessionToken, PlayerId>,
    disconnected_at_ms: BTreeMap<PlayerId, u64>,
    next_connection_id: u64,
}

#[derive(Clone, Debug)]
pub struct RegisteredSession {
    pub player_id: PlayerId,
    pub connection_id: ConnectionId,
    pub reconnect_token: SessionToken,
    pub reconnected: bool,
    pub reconnect_token_replaced: bool,
}

impl SessionRegistry {
    pub fn connect(
        &mut self,
        name: String,
        token: Option<SessionToken>,
        tx: OutboundTx,
    ) -> RegisteredSession {
        let requested_reconnect = token.is_some();
        let display_name = normalize_display_name(name);

        if let Some(token) = token {
            if let Some(player_id) = self.tokens.get(&token).cloned() {
                let connection_id = self.next_connection_id();
                if let Some(previous_tx) = self.sessions.insert(player_id.clone(), tx) {
                    let _ = broadcast::send(
                        &previous_tx,
                        ServerMessage::Event {
                            event: ServerEvent::SessionReplaced {
                                message: "This session was opened in another tab or client."
                                    .to_owned(),
                            },
                        },
                    );
                }
                self.connection_ids.insert(player_id.clone(), connection_id);
                self.disconnected_at_ms.remove(&player_id);
                self.player_names
                    .entry(player_id.clone())
                    .or_insert(display_name);
                return RegisteredSession {
                    player_id,
                    connection_id,
                    reconnect_token: token,
                    reconnected: true,
                    reconnect_token_replaced: false,
                };
            }
        }

        let player_id = new_player_id();
        let reconnect_token = new_session_token();
        let connection_id = self.next_connection_id();
        self.sessions.insert(player_id.clone(), tx);
        self.connection_ids.insert(player_id.clone(), connection_id);
        self.player_names.insert(player_id.clone(), display_name);
        self.tokens
            .insert(reconnect_token.clone(), player_id.clone());
        RegisteredSession {
            player_id,
            connection_id,
            reconnect_token,
            reconnected: false,
            reconnect_token_replaced: requested_reconnect,
        }
    }

    pub fn can_accept_connection(&self, token: Option<&SessionToken>, max_clients: usize) -> bool {
        if token.and_then(|token| self.tokens.get(token)).is_some() {
            return true;
        }

        self.retained_count() < max_clients
    }

    pub fn disconnect_socket(
        &mut self,
        player_id: &PlayerId,
        connection_id: ConnectionId,
        now_ms: u64,
    ) -> bool {
        if self.connection_ids.get(player_id) != Some(&connection_id) {
            return false;
        }
        self.sessions.remove(player_id);
        self.connection_ids.remove(player_id);
        if self.player_names.contains_key(player_id) {
            self.disconnected_at_ms.insert(player_id.clone(), now_ms);
        }
        true
    }

    pub fn prune_abandoned(
        &mut self,
        now_ms: u64,
        ttl_ms: u64,
        protected_player_ids: &BTreeSet<PlayerId>,
    ) -> Vec<PlayerId> {
        let expired: Vec<PlayerId> = self
            .disconnected_at_ms
            .iter()
            .filter(|(player_id, disconnected_at_ms)| {
                !self.sessions.contains_key(*player_id)
                    && !protected_player_ids.contains(*player_id)
                    && now_ms.saturating_sub(**disconnected_at_ms) >= ttl_ms
            })
            .map(|(player_id, _)| player_id.clone())
            .collect();

        for player_id in &expired {
            self.remove_identity(player_id);
        }

        expired
    }

    pub fn force_disconnect_socket(&mut self, player_id: &PlayerId, now_ms: u64) -> bool {
        let removed = self.sessions.remove(player_id).is_some();
        if !removed {
            return false;
        }

        self.connection_ids.remove(player_id);
        if self.player_names.contains_key(player_id) {
            self.disconnected_at_ms.insert(player_id.clone(), now_ms);
        }
        true
    }

    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn retained_count(&self) -> usize {
        self.player_names.len()
    }

    pub fn sessions(&self) -> &BTreeMap<PlayerId, OutboundTx> {
        &self.sessions
    }

    pub fn player_name(&self, player_id: &PlayerId) -> Option<&str> {
        self.player_names.get(player_id).map(String::as_str)
    }

    pub fn player_name_or_id(&self, player_id: &PlayerId) -> String {
        self.player_name(player_id)
            .map(str::to_owned)
            .unwrap_or_else(|| player_id.as_str().to_owned())
    }

    pub fn rename_player(
        &mut self,
        player_id: &PlayerId,
        name: String,
    ) -> Result<String, CoreError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyName);
        }
        let Some(stored) = self.player_names.get_mut(player_id) else {
            return Err(CoreError::PlayerNotFound(player_id.clone()));
        };
        *stored = trimmed.to_owned();
        Ok(stored.clone())
    }

    fn remove_identity(&mut self, player_id: &PlayerId) {
        self.sessions.remove(player_id);
        self.connection_ids.remove(player_id);
        self.player_names.remove(player_id);
        self.disconnected_at_ms.remove(player_id);
        self.tokens.retain(|_, owner_id| owner_id != player_id);
    }

    fn next_connection_id(&mut self) -> ConnectionId {
        self.next_connection_id = self
            .next_connection_id
            .checked_add(1)
            .expect("connection id overflow");
        ConnectionId(self.next_connection_id)
    }
}

fn normalize_display_name(name: String) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        FALLBACK_DISPLAY_NAME.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use play_room_protocol::{ServerEvent, ServerMessage};

    #[test]
    fn known_token_reconnects_same_identity() {
        let (first_tx, _) = crate::broadcast::channel();
        let (second_tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let first = registry.connect("Alice".to_owned(), None, first_tx);
        registry.disconnect_socket(&first.player_id, first.connection_id, 1_000);
        let second = registry.connect(
            String::new(),
            Some(first.reconnect_token.clone()),
            second_tx,
        );

        assert_eq!(second.player_id, first.player_id);
        assert_eq!(second.reconnect_token, first.reconnect_token);
        assert!(second.reconnected);
        assert!(!second.reconnect_token_replaced);
        assert_eq!(registry.player_name(&first.player_id), Some("Alice"));
        assert_eq!(registry.active_count(), 1);
        assert_eq!(registry.retained_count(), 1);
    }

    #[test]
    fn stale_disconnect_after_active_reconnect_is_ignored() {
        let (first_tx, _) = crate::broadcast::channel();
        let (second_tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let first = registry.connect("Alice".to_owned(), None, first_tx);
        let second = registry.connect(
            String::new(),
            Some(first.reconnect_token.clone()),
            second_tx,
        );

        assert_eq!(second.player_id, first.player_id);
        assert_ne!(second.connection_id, first.connection_id);
        assert!(!registry.disconnect_socket(&first.player_id, first.connection_id, 1_000));
        assert_eq!(registry.active_count(), 1);
        assert_eq!(registry.player_name(&first.player_id), Some("Alice"));

        assert!(registry.disconnect_socket(&second.player_id, second.connection_id, 2_000));
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn active_reconnect_notifies_replaced_socket() {
        let (first_tx, mut first_rx) = crate::broadcast::channel();
        let (second_tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let first = registry.connect("Alice".to_owned(), None, first_tx);
        let second = registry.connect(
            String::new(),
            Some(first.reconnect_token.clone()),
            second_tx,
        );

        let message = first_rx
            .try_recv()
            .expect("replaced socket should receive a session replacement event");

        assert_eq!(second.player_id, first.player_id);
        assert!(matches!(
            message,
            ServerMessage::Event {
                event: ServerEvent::SessionReplaced { .. }
            }
        ));
    }

    #[test]
    fn unknown_token_creates_new_identity_and_replaces_token() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let missing = SessionToken::new("missing-token");
        let connected = registry.connect(" Alice ".to_owned(), Some(missing.clone()), tx);

        assert!(!connected.reconnected);
        assert!(connected.reconnect_token_replaced);
        assert_ne!(connected.reconnect_token, missing);
        assert_eq!(registry.player_name(&connected.player_id), Some("Alice"));
    }

    #[test]
    fn empty_new_session_name_uses_guest_fallback() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let connected = registry.connect("   ".to_owned(), None, tx);

        assert!(!connected.reconnected);
        assert!(!connected.reconnect_token_replaced);
        assert_eq!(registry.player_name(&connected.player_id), Some("Guest"));
    }

    #[test]
    fn connected_identity_can_be_renamed() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let connected = registry.connect("Alice".to_owned(), None, tx);

        let renamed = registry
            .rename_player(&connected.player_id, "  Alicia  ".to_owned())
            .unwrap();

        assert_eq!(renamed, "Alicia");
        assert_eq!(registry.player_name(&connected.player_id), Some("Alicia"));
    }

    #[test]
    fn rename_rejects_empty_name() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let connected = registry.connect("Alice".to_owned(), None, tx);

        let err = registry
            .rename_player(&connected.player_id, "  ".to_owned())
            .unwrap_err();

        assert_eq!(err, CoreError::EmptyName);
        assert_eq!(registry.player_name(&connected.player_id), Some("Alice"));
    }

    #[test]
    fn active_client_limit_allows_replacing_current_identity() {
        let (first_tx, _) = crate::broadcast::channel();
        let (second_tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let first = registry.connect("Alice".to_owned(), None, first_tx);

        assert!(registry.can_accept_connection(Some(&first.reconnect_token), 1));
        let second = registry.connect(
            String::new(),
            Some(first.reconnect_token.clone()),
            second_tx,
        );

        assert_eq!(second.player_id, first.player_id);
        assert_eq!(registry.active_count(), 1);
        assert_eq!(registry.retained_count(), 1);
    }

    #[test]
    fn retained_client_limit_rejects_new_identity_when_full() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        registry.connect("Alice".to_owned(), None, tx);

        assert!(!registry.can_accept_connection(None, 1));
        assert!(!registry.can_accept_connection(Some(&SessionToken::new("missing")), 1));
    }

    #[test]
    fn abandoned_disconnected_session_expires_after_ttl() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let connected = registry.connect("Alice".to_owned(), None, tx);
        registry.disconnect_socket(&connected.player_id, connected.connection_id, 1_000);

        let expired = registry.prune_abandoned(31_000, 30_000, &BTreeSet::new());

        assert_eq!(expired, vec![connected.player_id.clone()]);
        assert_eq!(registry.player_name(&connected.player_id), None);
        assert_eq!(registry.retained_count(), 0);
        assert!(registry.can_accept_connection(Some(&connected.reconnect_token), 1));
    }

    #[test]
    fn protected_disconnected_session_does_not_expire() {
        let (tx, _) = crate::broadcast::channel();
        let mut registry = SessionRegistry::default();
        let connected = registry.connect("Alice".to_owned(), None, tx);
        registry.disconnect_socket(&connected.player_id, connected.connection_id, 1_000);
        let protected = BTreeSet::from([connected.player_id.clone()]);

        let expired = registry.prune_abandoned(31_000, 30_000, &protected);

        assert!(expired.is_empty());
        assert_eq!(registry.player_name(&connected.player_id), Some("Alice"));
    }
}
