//! Deterministic game and room state machine for Play Room.
//!
//! This crate intentionally contains no networking, async runtime, JSON codecs,
//! filesystem access, or wall-clock sleeping. Server runtimes pass timestamps in
//! explicitly when applying timeout-related commands.

pub mod command;
pub mod errors;
pub mod event;
pub mod game;
pub mod ids;
pub mod player;
pub mod room;
pub mod rules;
pub mod scoreboard;
pub mod state;
pub mod timer;

pub use command::RoomCommand;
pub use errors::CoreError;
pub use event::RoomEvent;
pub use game::{compare_moves, GameKind, Move, RoundEndReason, RoundOutcome, RoundResult};
pub use ids::{PlayerId, RoomId, SessionToken};
pub use player::{Player, PlayerRole};
pub use room::GameRoom;
pub use rules::GameRules;
pub use scoreboard::PlayerScore;
pub use state::{PlayerView, RoomPhase, RoomSnapshot, RoomSummary};
pub use timer::Deadline;
