use crate::room_registry::RoomLookupError;
use play_room_core::CoreError;
use play_room_protocol::{ErrorCode, ServerResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoomManagerError {
    message: String,
    code: Option<ErrorCode>,
    suggestions: Vec<String>,
}

impl RoomManagerError {
    pub(super) fn plain(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            suggestions: Vec::new(),
        }
    }

    pub(super) fn coded(message: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
            suggestions: Vec::new(),
        }
    }

    pub(super) fn with_suggestions(
        message: impl Into<String>,
        code: ErrorCode,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
            suggestions,
        }
    }

    pub(super) fn room_not_found(room: impl std::fmt::Display) -> Self {
        Self::coded(format!("room not found: {room}"), ErrorCode::RoomNotFound)
    }

    pub(super) fn not_in_room() -> Self {
        Self::coded("player is not in a room", ErrorCode::NotInRoom)
    }

    pub(super) fn room_limit_reached(max_rooms: usize) -> Self {
        Self::coded(
            format!("room limit reached; max_rooms={max_rooms}"),
            ErrorCode::RoomLimitReached,
        )
    }

    pub(super) fn client_limit_reached(max_clients: usize) -> Self {
        Self::coded(
            format!("client limit reached; max_clients={max_clients}"),
            ErrorCode::ClientLimitReached,
        )
    }

    pub(super) fn duplicate_player_name(
        name: impl Into<String>,
        connected: Option<bool>,
        suggestions: Vec<String>,
    ) -> Self {
        let name = name.into();
        let message = match connected {
            Some(false) => format!(
                "{name} is already in this room but currently disconnected. Reconnect with the session token or choose another name."
            ),
            Some(true) => format!("{name} is already in this room. Choose another name."),
            None => format!("player name already exists in this room: {name}"),
        };
        Self::with_suggestions(message, ErrorCode::PlayerNameExists, suggestions)
    }

    pub(super) fn from_core(error: CoreError) -> Self {
        match error {
            CoreError::RoomNotFound(room_id) => Self::room_not_found(room_id),
            CoreError::RoomFull => Self::coded("room is full", ErrorCode::RoomFull),
            CoreError::DuplicatePlayerName(name) => {
                Self::duplicate_player_name(name, None, Vec::new())
            }
            CoreError::MatchNotFinished => {
                Self::coded("match is not finished", ErrorCode::MatchNotFinished)
            }
            CoreError::HostOnly => Self::coded(
                "only the room host can start the next match",
                ErrorCode::HostOnly,
            ),
            CoreError::AlreadyInRoom
            | CoreError::RoomFinished
            | CoreError::MatchInProgress
            | CoreError::SpectatorsNotAllowed
            | CoreError::SpectatorAction
            | CoreError::PlayerDisconnected
            | CoreError::RoundNotActive
            | CoreError::RoundExpired
            | CoreError::RoundAlreadyActive
            | CoreError::InvalidMove { .. }
            | CoreError::NotEnoughReadyParticipants
            | CoreError::StaleTimeout
            | CoreError::EmptyName
            | CoreError::InvalidRules(_)
            | CoreError::PlayerNotFound(_) => {
                Self::coded(error.to_string(), ErrorCode::InvalidAction)
            }
        }
    }
    pub(super) fn from_room_lookup(error: RoomLookupError) -> Self {
        match error {
            RoomLookupError::NotFound(requested) => Self::room_not_found(requested),
            RoomLookupError::Ambiguous(requested) => Self::coded(
                format!("multiple rooms named {requested}; use the room id from /rooms"),
                ErrorCode::InvalidRequest,
            ),
        }
    }
    #[cfg(test)]
    pub(super) fn message(&self) -> &str {
        &self.message
    }

    #[cfg(test)]
    pub(super) fn code(&self) -> Option<&ErrorCode> {
        self.code.as_ref()
    }

    #[cfg(test)]
    pub(super) fn suggestions(&self) -> &[String] {
        &self.suggestions
    }

    pub fn into_server_result(self) -> ServerResult {
        ServerResult::Error {
            message: self.message,
            code: self.code,
            suggestions: self.suggestions,
        }
    }
}
