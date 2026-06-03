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
        if self.min_players != 2 || self.max_players != 2 {
            return Err(CoreError::InvalidRules(
                "RPS/RPSLS rooms support exactly 2 active participants".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ids::PlayerId, ids::RoomId, player::Player, room::GameRoom};

    fn exact_two_participants_error() -> CoreError {
        CoreError::InvalidRules("RPS/RPSLS rooms support exactly 2 active participants".to_owned())
    }

    #[test]
    fn default_rules_are_valid_two_player_rps() {
        let rules = GameRules::default();

        assert_eq!(rules.game, GameKind::RockPaperScissors);
        assert_eq!(rules.min_players, 2);
        assert_eq!(rules.max_players, 2);
        assert_eq!(rules.target_score, 2);
        assert_eq!(rules.round_seconds, 15);
        assert!(rules.allow_spectators);
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn rpsls_rules_are_valid_two_player_rpsls() {
        let rules = GameRules::rpsls();

        assert_eq!(rules.game, GameKind::RockPaperScissorsLizardSpock);
        assert_eq!(rules.min_players, 2);
        assert_eq!(rules.max_players, 2);
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn best_of_rounds_maps_target_score_to_max_rounds() {
        let best_of_three = GameRules::default();
        let best_of_five = GameRules {
            target_score: 3,
            ..GameRules::default()
        };
        let invalid_zero = GameRules {
            target_score: 0,
            ..GameRules::default()
        };

        assert_eq!(best_of_three.best_of_rounds(), 3);
        assert_eq!(best_of_five.best_of_rounds(), 5);
        assert_eq!(invalid_zero.best_of_rounds(), 0);
    }

    #[test]
    fn rules_reject_max_players_below_exact_participant_count() {
        let rules = GameRules {
            min_players: 2,
            max_players: 1,
            ..GameRules::default()
        };

        assert_eq!(
            rules.validate().unwrap_err(),
            exact_two_participants_error()
        );
    }

    #[test]
    fn rules_reject_more_than_two_participants() {
        let rules = GameRules {
            min_players: 3,
            max_players: 3,
            ..GameRules::default()
        };

        assert_eq!(
            rules.validate().unwrap_err(),
            exact_two_participants_error()
        );
    }

    #[test]
    fn rules_reject_zero_target_score() {
        let rules = GameRules {
            target_score: 0,
            ..GameRules::default()
        };

        assert_eq!(
            rules.validate().unwrap_err(),
            CoreError::InvalidRules("target_score must be at least 1".to_owned())
        );
    }

    #[test]
    fn rules_reject_zero_round_seconds() {
        let rules = GameRules {
            round_seconds: 0,
            ..GameRules::default()
        };

        assert_eq!(
            rules.validate().unwrap_err(),
            CoreError::InvalidRules("round_seconds must be at least 1".to_owned())
        );
    }

    #[test]
    fn game_room_new_rejects_invalid_rules() {
        let rules = GameRules {
            max_players: 1,
            ..GameRules::default()
        };
        let host = Player::participant(PlayerId::new("alice"), "Alice");

        let err = GameRoom::new(RoomId::new("room"), "room", rules, host).unwrap_err();

        assert_eq!(err, exact_two_participants_error());
    }
}
