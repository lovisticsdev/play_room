use crate::broadcast::OutboundTx;
use crate::identity::{new_player_id, new_session_token};
use play_room_core::{PlayerId, SessionToken};
use std::collections::BTreeMap;

const FALLBACK_DISPLAY_NAME: &str = "Guest";

#[derive(Default)]
pub struct SessionRegistry {
    sessions: BTreeMap<PlayerId, OutboundTx>,
    player_names: BTreeMap<PlayerId, String>,
    tokens: BTreeMap<SessionToken, PlayerId>,
}

#[derive(Clone, Debug)]
pub struct RegisteredSession {
    pub player_id: PlayerId,
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
                self.sessions.insert(player_id.clone(), tx);
                self.player_names
                    .entry(player_id.clone())
                    .or_insert(display_name);
                return RegisteredSession {
                    player_id,
                    reconnect_token: token,
                    reconnected: true,
                    reconnect_token_replaced: false,
                };
            }
        }

        let player_id = new_player_id();
        let reconnect_token = new_session_token();
        self.sessions.insert(player_id.clone(), tx);
        self.player_names.insert(player_id.clone(), display_name);
        self.tokens
            .insert(reconnect_token.clone(), player_id.clone());
        RegisteredSession {
            player_id,
            reconnect_token,
            reconnected: false,
            reconnect_token_replaced: requested_reconnect,
        }
    }

    pub fn disconnect_socket(&mut self, player_id: &PlayerId) {
        self.sessions.remove(player_id);
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

    #[test]
    fn known_token_reconnects_same_identity() {
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
        assert_eq!(second.reconnect_token, first.reconnect_token);
        assert!(second.reconnected);
        assert!(!second.reconnect_token_replaced);
        assert_eq!(registry.player_name(&first.player_id), Some("Alice"));
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
}
