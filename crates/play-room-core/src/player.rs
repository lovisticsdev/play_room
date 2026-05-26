use crate::ids::PlayerId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerRole {
    Participant,
    Spectator,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub role: PlayerRole,
    pub ready: bool,
    pub connected: bool,
    pub score: u32,
}

impl Player {
    pub fn participant(id: PlayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            role: PlayerRole::Participant,
            ready: false,
            connected: true,
            score: 0,
        }
    }

    pub fn spectator(id: PlayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            role: PlayerRole::Spectator,
            ready: false,
            connected: true,
            score: 0,
        }
    }

    pub fn is_active_participant(&self) -> bool {
        self.connected && self.role == PlayerRole::Participant
    }
}
