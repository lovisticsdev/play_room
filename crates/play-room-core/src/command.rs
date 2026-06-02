use crate::game::Move;
use crate::ids::PlayerId;
use crate::player::Player;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "command")]
pub enum RoomCommand {
    Join {
        player: Player,
    },
    Leave {
        player_id: PlayerId,
    },
    SetReady {
        player_id: PlayerId,
        ready: bool,
        now_ms: u64,
    },
    SetSpectator {
        player_id: PlayerId,
        spectator: bool,
    },
    SubmitMove {
        player_id: PlayerId,
        mv: Move,
        now_ms: u64,
    },
    Disconnect {
        player_id: PlayerId,
    },
    Reconnect {
        player_id: PlayerId,
    },
    TimeoutRound {
        round: u32,
        now_ms: u64,
    },
    ExpireParticipantSeat {
        player_id: PlayerId,
    },
    StartNextMatch {
        player_id: PlayerId,
    },
}
