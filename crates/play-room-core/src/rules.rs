use crate::game::GameKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GameRules {
    pub game: GameKind,
    pub min_players: usize,
    pub max_players: usize,
    pub target_score: u32,
    pub round_seconds: u64,
    pub allow_spectators: bool,
}

impl GameRules {
    pub fn rps() -> Self {
        Self {
            game: GameKind::RockPaperScissors,
            min_players: 2,
            max_players: 2,
            target_score: 3,
            round_seconds: 15,
            allow_spectators: true,
        }
    }

    pub fn rpsls() -> Self {
        Self {
            game: GameKind::RockPaperScissorsLizardSpock,
            min_players: 2,
            max_players: 2,
            target_score: 3,
            round_seconds: 15,
            allow_spectators: true,
        }
    }
}

impl Default for GameRules {
    fn default() -> Self {
        Self::rps()
    }
}
