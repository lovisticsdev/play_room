use play_room_core::{Move, RoomId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "step")]
pub enum ScenarioStep {
    Connect { client: String },
    CreateRoom { client: String, name: String },
    JoinRoom { client: String, room_id: RoomId },
    Ready { client: String },
    Move { client: String, mv: Move },
    Spectate { client: String },
    WaitMs { ms: u64 },
    Disconnect { client: String },
    Reconnect { client: String },
}
