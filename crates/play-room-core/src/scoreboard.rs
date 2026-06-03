use crate::ids::PlayerId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlayerScore {
    pub player_id: PlayerId,
    pub name: String,
    pub score: u32,
}
