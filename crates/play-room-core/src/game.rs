use crate::errors::CoreError;
use crate::ids::PlayerId;
use crate::scoreboard::PlayerScore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameKind {
    RockPaperScissors,
    RockPaperScissorsLizardSpock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Move {
    Rock,
    Paper,
    Scissors,
    Lizard,
    Spock,
}

impl Move {
    pub fn valid_for(self, game: GameKind) -> bool {
        match game {
            GameKind::RockPaperScissors => {
                matches!(self, Move::Rock | Move::Paper | Move::Scissors)
            }
            GameKind::RockPaperScissorsLizardSpock => true,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "rock" | "r" => Some(Move::Rock),
            "paper" | "p" => Some(Move::Paper),
            "scissors" | "scissor" | "s" => Some(Move::Scissors),
            "lizard" | "l" => Some(Move::Lizard),
            "spock" | "sp" => Some(Move::Spock),
            _ => None,
        }
    }

    fn beats(self, other: Move) -> bool {
        matches!(
            (self, other),
            (Move::Rock, Move::Scissors)
                | (Move::Rock, Move::Lizard)
                | (Move::Paper, Move::Rock)
                | (Move::Paper, Move::Spock)
                | (Move::Scissors, Move::Paper)
                | (Move::Scissors, Move::Lizard)
                | (Move::Lizard, Move::Spock)
                | (Move::Lizard, Move::Paper)
                | (Move::Spock, Move::Scissors)
                | (Move::Spock, Move::Rock)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundOutcome {
    Draw,
    Win { winner: PlayerId },
    TimeoutWin { winner: PlayerId },
    NoContest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundEndReason {
    AllMovesSubmitted,
    Timeout,
    PlayerLeft,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoundResult {
    pub round: u32,
    pub reason: RoundEndReason,
    pub submitted: BTreeMap<PlayerId, Option<Move>>,
    pub outcome: RoundOutcome,
    pub scores: Vec<PlayerScore>,
}

pub fn compare_moves(
    game: GameKind,
    left: Move,
    right: Move,
) -> Result<std::cmp::Ordering, CoreError> {
    if !left.valid_for(game) {
        return Err(CoreError::InvalidMove { game, mv: left });
    }
    if !right.valid_for(game) {
        return Err(CoreError::InvalidMove { game, mv: right });
    }

    if left == right {
        Ok(std::cmp::Ordering::Equal)
    } else if left.beats(right) {
        Ok(std::cmp::Ordering::Greater)
    } else {
        Ok(std::cmp::Ordering::Less)
    }
}
