use play_room_core::{GameRules, Move, RoomId, SessionToken};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnterRoomMode {
    Auto,
    Participant,
    Spectator,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ClientRequest {
    Connect {
        name: String,
        reconnect_token: Option<SessionToken>,
    },
    ListRooms,
    CreateRoom {
        name: String,
        rules: Option<GameRules>,
    },
    JoinRoom {
        room_id: RoomId,
    },
    SpectateRoom {
        room_id: RoomId,
    },
    EnterRoom {
        room_id: RoomId,
        mode: EnterRoomMode,
    },
    UpdateDisplayName {
        name: String,
    },
    UpdateMatchFormat {
        target_score: u32,
    },
    LeaveRoom,
    StartNextMatch,
    SetReady {
        ready: bool,
    },
    SetSpectator {
        spectator: bool,
    },
    SubmitMove {
        mv: Move,
    },
    Ping,
}
