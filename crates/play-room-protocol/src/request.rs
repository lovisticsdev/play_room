use play_room_core::{GameRules, Move, RoomId, SessionToken};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    LeaveRoom,
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
