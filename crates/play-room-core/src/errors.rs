use crate::game::{GameKind, Move};
use crate::ids::{PlayerId, RoomId};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("room not found: {0}")]
    RoomNotFound(RoomId),

    #[error("player not found: {0}")]
    PlayerNotFound(PlayerId),

    #[error("room is full")]
    RoomFull,

    #[error("room is already finished")]
    RoomFinished,

    #[error("match is not finished")]
    MatchNotFinished,

    #[error("only the room host can perform this action")]
    HostOnly,

    #[error("invalid game rules: {0}")]
    InvalidRules(String),

    #[error("player name is empty")]
    EmptyName,

    #[error("player is already in room")]
    AlreadyInRoom,

    #[error("player name already exists in this room: {0}")]
    DuplicatePlayerName(String),

    #[error("spectators are not allowed in this room")]
    SpectatorsNotAllowed,

    #[error("spectator cannot perform this action")]
    SpectatorAction,

    #[error("player is not connected")]
    PlayerDisconnected,

    #[error("round is not active")]
    RoundNotActive,

    #[error("round is already active")]
    RoundAlreadyActive,

    #[error("move {mv:?} is invalid for game {game:?}")]
    InvalidMove { game: GameKind, mv: Move },

    #[error("not enough ready participants")]
    NotEnoughReadyParticipants,

    #[error("timeout does not match the active round")]
    StaleTimeout,
}
