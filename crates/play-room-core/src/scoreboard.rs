use crate::ids::PlayerId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerScore {
    pub player_id: PlayerId,
    pub name: String,
    pub score: u32,
}
