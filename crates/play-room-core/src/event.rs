use crate::game::RoundResult;
use crate::ids::PlayerId;
use crate::player::PlayerRole;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum RoomEvent {
    PlayerJoined {
        player_id: PlayerId,
        name: String,
        role: PlayerRole,
    },
    PlayerLeft {
        player_id: PlayerId,
    },
    PlayerDisconnected {
        player_id: PlayerId,
    },
    PlayerReconnected {
        player_id: PlayerId,
    },
    ReadyChanged {
        player_id: PlayerId,
        ready: bool,
    },
    RoleChanged {
        player_id: PlayerId,
        role: PlayerRole,
    },
    RoundStarted {
        round: u32,
        deadline_ms: u64,
    },
    MoveAccepted {
        player_id: PlayerId,
    },
    RoundResolved {
        result: RoundResult,
    },
    GameEnded {
        winner: Option<PlayerId>,
    },
    MatchReset {
        requested_by: PlayerId,
    },
    HostChanged {
        host_id: Option<PlayerId>,
    },
}
