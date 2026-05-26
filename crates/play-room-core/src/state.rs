use crate::ids::{PlayerId, RoomId};
use crate::player::PlayerRole;
use crate::rules::GameRules;
use crate::scoreboard::PlayerScore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "phase")]
pub enum RoomPhase {
    Lobby,
    InRound { round: u32, deadline_ms: u64 },
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    pub id: PlayerId,
    pub name: String,
    pub role: PlayerRole,
    pub ready: bool,
    pub connected: bool,
    pub score: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoomSummary {
    pub id: RoomId,
    pub name: String,
    pub phase: RoomPhase,
    pub players: usize,
    pub spectators: usize,
    pub max_players: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub id: RoomId,
    pub name: String,
    pub host_id: Option<PlayerId>,
    pub phase: RoomPhase,
    pub rules: GameRules,
    pub round: u32,
    pub players: Vec<PlayerView>,
    pub scoreboard: Vec<PlayerScore>,
}
