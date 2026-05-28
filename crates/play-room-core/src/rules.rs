use crate::{errors::CoreError, game::GameKind};
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
            target_score: 2,
            round_seconds: 15,
            allow_spectators: true,
        }
    }

    pub fn rpsls() -> Self {
        Self {
            game: GameKind::RockPaperScissorsLizardSpock,
            min_players: 2,
            max_players: 2,
            target_score: 2,
            round_seconds: 15,
            allow_spectators: true,
        }
    }

    pub fn best_of_rounds(&self) -> u32 {
        self.target_score.saturating_mul(2).saturating_sub(1)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.min_players < 2 {
            return Err(CoreError::InvalidRules(
                "min_players must be at least 2".to_owned(),
            ));
        }
        if self.max_players < self.min_players {
            return Err(CoreError::InvalidRules(
                "max_players must be greater than or equal to min_players".to_owned(),
            ));
        }
        if self.target_score == 0 {
            return Err(CoreError::InvalidRules(
                "target_score must be at least 1".to_owned(),
            ));
        }
        if self.round_seconds == 0 {
            return Err(CoreError::InvalidRules(
                "round_seconds must be at least 1".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Default for GameRules {
    fn default() -> Self {
        Self::rps()
    }
}
