use play_room_core::{PlayerId, RoomSnapshot, RoomSummary, SessionToken};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    RoomNotFound,
    RoomNameExists,
    PlayerNameExists,
    RoomFull,
    NotInRoom,
    MatchNotFinished,
    HostOnly,
    InvalidAction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum ServerResult {
    Ok,
    Error {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<ErrorCode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        suggestions: Vec<String>,
    },
    Welcome {
        player_id: PlayerId,
        reconnect_token: SessionToken,
        protocol_version: u16,
    },
    RoomList {
        rooms: Vec<RoomSummary>,
    },
    RoomSnapshot {
        room: RoomSnapshot,
    },
    Pong,
}

impl ServerResult {
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            code: None,
            suggestions: Vec::new(),
        }
    }

    pub fn coded_error(message: impl Into<String>, code: ErrorCode) -> Self {
        Self::Error {
            message: message.into(),
            code: Some(code),
            suggestions: Vec::new(),
        }
    }

    pub fn suggested_error(
        message: impl Into<String>,
        code: ErrorCode,
        suggestions: Vec<String>,
    ) -> Self {
        Self::Error {
            message: message.into(),
            code: Some(code),
            suggestions,
        }
    }
}
