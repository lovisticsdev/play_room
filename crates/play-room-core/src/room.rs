use crate::command::RoomCommand;
use crate::errors::CoreError;
use crate::event::RoomEvent;
use crate::game::{compare_moves, Move, RoundEndReason, RoundOutcome, RoundResult};
use crate::ids::{PlayerId, RoomId};
use crate::player::{Player, PlayerRole};
use crate::rules::GameRules;
use crate::scoreboard::PlayerScore;
use crate::state::{PlayerView, RoomPhase, RoomSnapshot, RoomSummary};
use crate::timer::Deadline;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct GameRoom {
    id: RoomId,
    name: String,
    rules: GameRules,
    host_id: Option<PlayerId>,
    phase: RoomPhase,
    round: u32,
    players: BTreeMap<PlayerId, Player>,
    moves: BTreeMap<PlayerId, Move>,
}

impl GameRoom {
    pub fn new(
        id: RoomId,
        name: impl Into<String>,
        rules: GameRules,
        host: Player,
    ) -> Result<Self, CoreError> {
        if host.name.trim().is_empty() {
            return Err(CoreError::EmptyName);
        }
        rules.validate()?;
        let host_id = host.id.clone();
        let mut players = BTreeMap::new();
        players.insert(host.id.clone(), host);
        Ok(Self {
            id,
            name: name.into(),
            rules,
            host_id: Some(host_id),
            phase: RoomPhase::Lobby,
            round: 0,
            players,
            moves: BTreeMap::new(),
        })
    }

    pub fn id(&self) -> &RoomId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn phase(&self) -> &RoomPhase {
        &self.phase
    }

    pub fn player_ids(&self) -> Vec<PlayerId> {
        self.players.keys().cloned().collect()
    }

    pub fn active_participant_ids(&self) -> Vec<PlayerId> {
        self.players
            .values()
            .filter(|p| p.is_active_participant())
            .map(|p| p.id.clone())
            .collect()
    }

    pub fn apply(&mut self, command: RoomCommand) -> Result<Vec<RoomEvent>, CoreError> {
        match command {
            RoomCommand::Join { player } => self.join(player),
            RoomCommand::Leave { player_id } => self.leave(&player_id),
            RoomCommand::SetReady {
                player_id,
                ready,
                now_ms,
            } => self.set_ready(&player_id, ready, now_ms),
            RoomCommand::SetSpectator {
                player_id,
                spectator,
            } => self.set_spectator(&player_id, spectator),
            RoomCommand::SubmitMove {
                player_id,
                mv,
                now_ms,
            } => self.submit_move(&player_id, mv, now_ms),
            RoomCommand::Disconnect { player_id } => self.disconnect(&player_id),
            RoomCommand::Reconnect { player_id } => self.reconnect(&player_id),
            RoomCommand::TimeoutRound { round, now_ms } => self.timeout_round(round, now_ms),
            RoomCommand::StartNextMatch { player_id } => self.start_next_match(&player_id),
        }
    }

    fn join(&mut self, player: Player) -> Result<Vec<RoomEvent>, CoreError> {
        let player_name = player.name.trim();
        if player_name.is_empty() {
            return Err(CoreError::EmptyName);
        }
        if self.players.contains_key(&player.id) {
            return Err(CoreError::AlreadyInRoom);
        }
        if self
            .players
            .values()
            .any(|existing| existing.name.trim().eq_ignore_ascii_case(player_name))
        {
            return Err(CoreError::DuplicatePlayerName(player_name.to_owned()));
        }
        if player.role == PlayerRole::Spectator && !self.rules.allow_spectators {
            return Err(CoreError::SpectatorsNotAllowed);
        }
        if player.role == PlayerRole::Participant
            && self.participant_count() >= self.rules.max_players
        {
            return Err(CoreError::RoomFull);
        }

        let event = RoomEvent::PlayerJoined {
            player_id: player.id.clone(),
            name: player.name.clone(),
            role: player.role,
        };
        self.players.insert(player.id.clone(), player);
        Ok(vec![event])
    }

    fn leave(&mut self, player_id: &PlayerId) -> Result<Vec<RoomEvent>, CoreError> {
        self.players
            .remove(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        self.moves.remove(player_id);
        let mut events = vec![RoomEvent::PlayerLeft {
            player_id: player_id.clone(),
        }];
        if self.host_id.as_ref() == Some(player_id) {
            self.host_id = self.next_host_id();
            events.push(RoomEvent::HostChanged {
                host_id: self.host_id.clone(),
            });
        }
        if matches!(self.phase, RoomPhase::InRound { .. })
            && self.active_participant_ids().len() < self.rules.min_players
        {
            events.extend(self.resolve_round(RoundEndReason::PlayerLeft)?);
        }
        Ok(events)
    }

    fn set_ready(
        &mut self,
        player_id: &PlayerId,
        ready: bool,
        now_ms: u64,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        if matches!(self.phase, RoomPhase::Finished { .. }) {
            return Err(CoreError::RoomFinished);
        }
        if matches!(self.phase, RoomPhase::InRound { .. }) {
            return Err(CoreError::RoundAlreadyActive);
        }
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        if player.role == PlayerRole::Spectator {
            return Err(CoreError::SpectatorAction);
        }
        if !player.connected {
            return Err(CoreError::PlayerDisconnected);
        }
        player.ready = ready;
        let mut events = vec![RoomEvent::ReadyChanged {
            player_id: player_id.clone(),
            ready,
        }];
        if self.can_start_round() {
            events.extend(self.start_round(now_ms)?);
        }
        Ok(events)
    }

    fn start_next_match(&mut self, player_id: &PlayerId) -> Result<Vec<RoomEvent>, CoreError> {
        if !matches!(self.phase, RoomPhase::Finished { .. }) {
            return Err(CoreError::MatchNotFinished);
        }
        let player = self
            .players
            .get(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        if !player.connected {
            return Err(CoreError::PlayerDisconnected);
        }
        if self.host_id.as_ref() != Some(player_id) {
            return Err(CoreError::HostOnly);
        }

        self.phase = RoomPhase::Lobby;
        self.round = 0;
        self.moves.clear();
        for player in self.players.values_mut() {
            player.score = 0;
            player.ready = false;
        }
        Ok(vec![RoomEvent::MatchReset {
            requested_by: player_id.clone(),
        }])
    }

    fn set_spectator(
        &mut self,
        player_id: &PlayerId,
        spectator: bool,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        if spectator && !self.rules.allow_spectators {
            return Err(CoreError::SpectatorsNotAllowed);
        }
        if matches!(self.phase, RoomPhase::InRound { .. }) {
            return Err(CoreError::RoundAlreadyActive);
        }
        let participants = self.participant_count();
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        let new_role = if spectator {
            PlayerRole::Spectator
        } else {
            PlayerRole::Participant
        };
        if new_role == PlayerRole::Participant
            && player.role == PlayerRole::Spectator
            && participants >= self.rules.max_players
        {
            return Err(CoreError::RoomFull);
        }
        player.role = new_role;
        player.ready = false;
        Ok(vec![RoomEvent::RoleChanged {
            player_id: player_id.clone(),
            role: new_role,
        }])
    }

    fn submit_move(
        &mut self,
        player_id: &PlayerId,
        mv: Move,
        _now_ms: u64,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        if !mv.valid_for(self.rules.game) {
            return Err(CoreError::InvalidMove {
                game: self.rules.game,
                mv,
            });
        }
        if !matches!(self.phase, RoomPhase::InRound { .. }) {
            return Err(CoreError::RoundNotActive);
        }
        let player = self
            .players
            .get(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        if player.role == PlayerRole::Spectator {
            return Err(CoreError::SpectatorAction);
        }
        if !player.connected {
            return Err(CoreError::PlayerDisconnected);
        }

        self.moves.insert(player_id.clone(), mv);
        let mut events = vec![RoomEvent::MoveAccepted {
            player_id: player_id.clone(),
        }];
        if self.all_active_moves_submitted() {
            events.extend(self.resolve_round(RoundEndReason::AllMovesSubmitted)?);
        }
        Ok(events)
    }

    fn disconnect(&mut self, player_id: &PlayerId) -> Result<Vec<RoomEvent>, CoreError> {
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        player.connected = false;
        player.ready = false;
        let mut events = vec![RoomEvent::PlayerDisconnected {
            player_id: player_id.clone(),
        }];
        if matches!(self.phase, RoomPhase::InRound { .. })
            && self.active_participant_ids().len() < self.rules.min_players
        {
            events.extend(self.resolve_round(RoundEndReason::PlayerLeft)?);
        }
        Ok(events)
    }

    fn reconnect(&mut self, player_id: &PlayerId) -> Result<Vec<RoomEvent>, CoreError> {
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        player.connected = true;
        Ok(vec![RoomEvent::PlayerReconnected {
            player_id: player_id.clone(),
        }])
    }

    fn timeout_round(&mut self, round: u32, now_ms: u64) -> Result<Vec<RoomEvent>, CoreError> {
        match self.phase {
            RoomPhase::InRound {
                round: active_round,
                deadline_ms,
            } => {
                if active_round != round || now_ms < deadline_ms {
                    return Err(CoreError::StaleTimeout);
                }
                self.resolve_round(RoundEndReason::Timeout)
            }
            _ => Err(CoreError::RoundNotActive),
        }
    }

    fn can_start_round(&self) -> bool {
        if !matches!(self.phase, RoomPhase::Lobby) {
            return false;
        }
        let active: Vec<&Player> = self
            .players
            .values()
            .filter(|p| p.is_active_participant())
            .collect();
        active.len() >= self.rules.min_players
            && active.len() <= self.rules.max_players
            && active.iter().all(|p| p.ready)
    }

    fn start_round(&mut self, now_ms: u64) -> Result<Vec<RoomEvent>, CoreError> {
        if !self.can_start_round() {
            return Err(CoreError::NotEnoughReadyParticipants);
        }
        self.round += 1;
        self.moves.clear();
        let deadline = Deadline::from_now(now_ms, self.rules.round_seconds.saturating_mul(1000));
        self.phase = RoomPhase::InRound {
            round: self.round,
            deadline_ms: deadline.expires_at_ms,
        };
        Ok(vec![RoomEvent::RoundStarted {
            round: self.round,
            deadline_ms: deadline.expires_at_ms,
        }])
    }

    fn resolve_round(&mut self, reason: RoundEndReason) -> Result<Vec<RoomEvent>, CoreError> {
        let round = self.round;
        let active = self.active_participant_ids();
        let mut submitted = BTreeMap::new();
        for player_id in &active {
            submitted.insert(player_id.clone(), self.moves.get(player_id).copied());
        }

        let outcome = self.round_outcome(&active, reason)?;
        if let RoundOutcome::Win { winner } | RoundOutcome::TimeoutWin { winner } = &outcome {
            if let Some(player) = self.players.get_mut(winner) {
                player.score = player.score.saturating_add(1);
            }
        }

        let scores = self.scoreboard();
        let result = RoundResult {
            round,
            reason,
            submitted,
            outcome: outcome.clone(),
            scores,
        };
        let mut events = vec![RoomEvent::RoundResolved { result }];
        self.moves.clear();

        let winner = self
            .players
            .values()
            .find(|p| p.score >= self.rules.target_score)
            .map(|p| p.id.clone());
        for player in self.players.values_mut() {
            player.ready = false;
        }
        if winner.is_some() {
            self.phase = RoomPhase::Finished {
                winner: winner.clone(),
            };
            events.push(RoomEvent::GameEnded { winner });
        } else {
            self.phase = RoomPhase::Lobby;
        }
        Ok(events)
    }

    fn round_outcome(
        &self,
        active: &[PlayerId],
        reason: RoundEndReason,
    ) -> Result<RoundOutcome, CoreError> {
        if active.len() != 2 {
            return Ok(RoundOutcome::NoContest);
        }
        let left = &active[0];
        let right = &active[1];
        let left_move = self.moves.get(left).copied();
        let right_move = self.moves.get(right).copied();

        match (left_move, right_move, reason) {
            (Some(a), Some(b), _) => match compare_moves(self.rules.game, a, b)? {
                Ordering::Equal => Ok(RoundOutcome::Draw),
                Ordering::Greater => Ok(RoundOutcome::Win {
                    winner: left.clone(),
                }),
                Ordering::Less => Ok(RoundOutcome::Win {
                    winner: right.clone(),
                }),
            },
            (Some(_), None, RoundEndReason::Timeout) => Ok(RoundOutcome::TimeoutWin {
                winner: left.clone(),
            }),
            (None, Some(_), RoundEndReason::Timeout) => Ok(RoundOutcome::TimeoutWin {
                winner: right.clone(),
            }),
            _ => Ok(RoundOutcome::NoContest),
        }
    }

    fn all_active_moves_submitted(&self) -> bool {
        let active = self.active_participant_ids();
        !active.is_empty() && active.iter().all(|id| self.moves.contains_key(id))
    }

    pub fn player_named(&self, name: &str) -> Option<PlayerView> {
        let requested = name.trim();
        self.players
            .values()
            .find(|player| player.name.trim().eq_ignore_ascii_case(requested))
            .map(|player| PlayerView {
                id: player.id.clone(),
                name: player.name.clone(),
                role: player.role,
                ready: player.ready,
                connected: player.connected,
                score: player.score,
            })
    }

    fn next_host_id(&self) -> Option<PlayerId> {
        self.players
            .values()
            .find(|player| player.connected && player.role == PlayerRole::Participant)
            .or_else(|| {
                self.players
                    .values()
                    .find(|player| player.role == PlayerRole::Participant)
            })
            .or_else(|| self.players.values().find(|player| player.connected))
            .or_else(|| self.players.values().next())
            .map(|player| player.id.clone())
    }

    fn participant_count(&self) -> usize {
        self.players
            .values()
            .filter(|p| p.role == PlayerRole::Participant)
            .count()
    }

    pub fn scoreboard(&self) -> Vec<PlayerScore> {
        let mut scores: Vec<PlayerScore> = self
            .players
            .values()
            .filter(|p| p.role == PlayerRole::Participant)
            .map(|p| PlayerScore {
                player_id: p.id.clone(),
                name: p.name.clone(),
                score: p.score,
            })
            .collect();
        scores.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
        scores
    }

    pub fn summary(&self) -> RoomSummary {
        RoomSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            phase: self.phase.clone(),
            players: self
                .players
                .values()
                .filter(|p| p.role == PlayerRole::Participant)
                .count(),
            spectators: self
                .players
                .values()
                .filter(|p| p.role == PlayerRole::Spectator)
                .count(),
            max_players: self.rules.max_players,
            game: self.rules.game,
            target_score: self.rules.target_score,
        }
    }

    pub fn snapshot(&self) -> RoomSnapshot {
        let players = self
            .players
            .values()
            .map(|p| PlayerView {
                id: p.id.clone(),
                name: p.name.clone(),
                role: p.role,
                ready: p.ready,
                connected: p.connected,
                score: p.score,
            })
            .collect();

        RoomSnapshot {
            id: self.id.clone(),
            name: self.name.clone(),
            host_id: self.host_id.clone(),
            phase: self.phase.clone(),
            rules: self.rules.clone(),
            round: self.round,
            players,
            scoreboard: self.scoreboard(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn two_player_room() -> GameRoom {
        let host = Player::participant(PlayerId::new("alice"), "Alice");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
        room.apply(RoomCommand::Join {
            player: Player::participant(PlayerId::new("bob"), "Bob"),
        })
        .unwrap();
        room
    }

    fn ready(room: &mut GameRoom, player_id: &str, now_ms: u64) {
        room.apply(RoomCommand::SetReady {
            player_id: PlayerId::new(player_id),
            ready: true,
            now_ms,
        })
        .unwrap();
    }

    fn alice_wins_round(room: &mut GameRoom, now_ms: u64) {
        ready(room, "alice", now_ms);
        ready(room, "bob", now_ms);
        room.apply(RoomCommand::SubmitMove {
            player_id: PlayerId::new("alice"),
            mv: Move::Paper,
            now_ms: now_ms + 1,
        })
        .unwrap();
        room.apply(RoomCommand::SubmitMove {
            player_id: PlayerId::new("bob"),
            mv: Move::Rock,
            now_ms: now_ms + 2,
        })
        .unwrap();
    }

    #[test]
    fn scoreboard_excludes_spectators_but_keeps_disconnected_participants() {
        let host_id = PlayerId::new("host");
        let guest_id = PlayerId::new("guest");
        let spectator_id = PlayerId::new("spectator");
        let host = Player::participant(host_id.clone(), "Host");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

        room.apply(RoomCommand::Join {
            player: Player::participant(guest_id.clone(), "Guest"),
        })
        .unwrap();
        room.apply(RoomCommand::Join {
            player: Player::spectator(spectator_id.clone(), "Spectator"),
        })
        .unwrap();
        room.apply(RoomCommand::Disconnect {
            player_id: guest_id.clone(),
        })
        .unwrap();

        let scores = room.scoreboard();

        assert!(scores.iter().any(|score| score.player_id == host_id));
        assert!(scores.iter().any(|score| score.player_id == guest_id));
        assert!(!scores.iter().any(|score| score.player_id == spectator_id));
    }

    #[test]
    fn duplicate_player_names_are_rejected_within_a_room() {
        let host = Player::participant(PlayerId::new("host"), "Alex");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

        let err = room
            .apply(RoomCommand::Join {
                player: Player::participant(PlayerId::new("guest"), "alex"),
            })
            .unwrap_err();

        assert_eq!(err, CoreError::DuplicatePlayerName("alex".to_owned()));
    }

    #[test]
    fn best_of_three_finishes_when_a_player_reaches_two_points() {
        let mut room = two_player_room();

        alice_wins_round(&mut room, 1000);
        assert!(matches!(room.phase(), RoomPhase::Lobby));

        alice_wins_round(&mut room, 2000);

        assert!(matches!(
            room.phase(),
            RoomPhase::Finished {
                winner: Some(player_id)
            } if player_id == &PlayerId::new("alice")
        ));
        assert_eq!(room.snapshot().scoreboard[0].score, 2);
    }

    #[test]
    fn host_can_reset_finished_match_without_changing_seats() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);
        alice_wins_round(&mut room, 2000);

        let events = room
            .apply(RoomCommand::StartNextMatch {
                player_id: PlayerId::new("alice"),
            })
            .unwrap();

        assert!(matches!(room.phase(), RoomPhase::Lobby));
        assert!(matches!(events.as_slice(), [RoomEvent::MatchReset { .. }]));
        assert_eq!(room.snapshot().round, 0);
        assert!(room
            .snapshot()
            .players
            .iter()
            .all(|player| player.score == 0));
        assert!(room.snapshot().players.iter().all(|player| !player.ready));
        assert_eq!(room.participant_count(), 2);
    }

    #[test]
    fn non_host_cannot_reset_finished_match() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);
        alice_wins_round(&mut room, 2000);

        let err = room
            .apply(RoomCommand::StartNextMatch {
                player_id: PlayerId::new("bob"),
            })
            .unwrap_err();

        assert_eq!(err, CoreError::HostOnly);
    }

    #[test]
    fn host_transfer_prefers_connected_participants() {
        let host = Player::participant(PlayerId::new("host"), "Host");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
        room.apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("spectator"), "Spectator"),
        })
        .unwrap();
        room.apply(RoomCommand::Join {
            player: Player::participant(PlayerId::new("guest"), "Guest"),
        })
        .unwrap();

        room.apply(RoomCommand::Leave {
            player_id: PlayerId::new("host"),
        })
        .unwrap();

        assert_eq!(room.snapshot().host_id, Some(PlayerId::new("guest")));
    }
}
