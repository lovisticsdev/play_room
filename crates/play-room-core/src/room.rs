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
            RoomCommand::RenamePlayer { player_id, name } => self.rename_player(&player_id, name),
            RoomCommand::UpdateMatchFormat {
                player_id,
                target_score,
            } => self.update_match_format(&player_id, target_score),
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
            RoomCommand::ExpireParticipantSeat { player_id } => {
                self.expire_participant_seat(&player_id)
            }
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
        if player.role == PlayerRole::Participant && self.match_in_progress() {
            return Err(CoreError::MatchInProgress);
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
        let was_participant = self
            .players
            .get(player_id)
            .map(|player| player.role == PlayerRole::Participant)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        let match_in_progress = self.match_in_progress();

        self.players
            .remove(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        self.moves.remove(player_id);
        let mut events = vec![RoomEvent::PlayerLeft {
            player_id: player_id.clone(),
        }];
        self.transfer_host_if_current(player_id, &mut events);
        if was_participant && match_in_progress {
            if matches!(self.phase, RoomPhase::InRound { .. }) {
                events.extend(self.resolve_round(RoundEndReason::PlayerLeft)?);
            } else {
                events.extend(self.finish_match_by_forfeit());
            }
            return Ok(events);
        }
        if matches!(self.phase, RoomPhase::InRound { .. })
            && self.active_participant_ids().len() < self.rules.min_players
        {
            events.extend(self.resolve_round(RoundEndReason::PlayerLeft)?);
        }
        Ok(events)
    }

    fn rename_player(
        &mut self,
        player_id: &PlayerId,
        name: String,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyName);
        }
        if self.players.values().any(|existing| {
            existing.id != *player_id && existing.name.trim().eq_ignore_ascii_case(trimmed)
        }) {
            return Err(CoreError::DuplicatePlayerName(trimmed.to_owned()));
        }

        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        if player.name == trimmed {
            return Ok(Vec::new());
        }

        player.name = trimmed.to_owned();
        Ok(vec![RoomEvent::PlayerRenamed {
            player_id: player_id.clone(),
            name: player.name.clone(),
        }])
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
        if player.ready == ready {
            return Ok(Vec::new());
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

    fn update_match_format(
        &mut self,
        player_id: &PlayerId,
        target_score: u32,
    ) -> Result<Vec<RoomEvent>, CoreError> {
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
        if self.match_in_progress() {
            return Err(CoreError::MatchInProgress);
        }

        let mut rules = self.rules.clone();
        rules.target_score = target_score;
        rules.validate()?;
        if self.rules.target_score == target_score {
            return Ok(Vec::new());
        }

        self.rules = rules;
        let mut events = vec![RoomEvent::MatchFormatChanged { target_score }];
        for player in self.players.values_mut() {
            if player.ready {
                player.ready = false;
                events.push(RoomEvent::ReadyChanged {
                    player_id: player.id.clone(),
                    ready: false,
                });
            }
        }
        Ok(events)
    }

    fn set_spectator(
        &mut self,
        player_id: &PlayerId,
        spectator: bool,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        if spectator && !self.rules.allow_spectators {
            return Err(CoreError::SpectatorsNotAllowed);
        }
        let new_role = if spectator {
            PlayerRole::Spectator
        } else {
            PlayerRole::Participant
        };
        let player = self
            .players
            .get(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        if !player.connected {
            return Err(CoreError::PlayerDisconnected);
        }
        if player.role == new_role {
            return Ok(Vec::new());
        }
        if matches!(self.phase, RoomPhase::InRound { .. }) {
            return Err(CoreError::RoundAlreadyActive);
        }
        if self.match_in_progress() {
            return Err(CoreError::MatchInProgress);
        }
        let participants = self.participant_count();
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
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
        now_ms: u64,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        if !mv.valid_for(self.rules.game) {
            return Err(CoreError::InvalidMove {
                game: self.rules.game,
                mv,
            });
        }
        let deadline = match self.phase {
            RoomPhase::InRound { deadline_ms, .. } => Deadline {
                expires_at_ms: deadline_ms,
            },
            _ => return Err(CoreError::RoundNotActive),
        };
        if deadline.is_expired(now_ms) {
            return Err(CoreError::RoundExpired);
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
        let was_participant = self
            .players
            .get(player_id)
            .map(|player| player.role == PlayerRole::Participant)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        let match_in_progress = self.match_in_progress();

        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
        player.connected = false;
        player.ready = false;
        let mut events = vec![RoomEvent::PlayerDisconnected {
            player_id: player_id.clone(),
        }];
        if was_participant && match_in_progress {
            self.transfer_host_if_current(player_id, &mut events);
            if matches!(self.phase, RoomPhase::InRound { .. }) {
                events.extend(self.resolve_round(RoundEndReason::PlayerLeft)?);
            } else {
                events.extend(self.finish_match_by_forfeit());
            }
            return Ok(events);
        }
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

    fn expire_participant_seat(
        &mut self,
        player_id: &PlayerId,
    ) -> Result<Vec<RoomEvent>, CoreError> {
        {
            let player = self
                .players
                .get_mut(player_id)
                .ok_or_else(|| CoreError::PlayerNotFound(player_id.clone()))?;
            if player.connected || player.role != PlayerRole::Participant {
                return Ok(Vec::new());
            }
            player.role = PlayerRole::Spectator;
            player.ready = false;
        }

        self.moves.remove(player_id);
        let mut events = vec![RoomEvent::RoleChanged {
            player_id: player_id.clone(),
            role: PlayerRole::Spectator,
        }];
        if self.host_id.as_ref() == Some(player_id) {
            self.host_id = self.next_host_id();
            events.push(RoomEvent::HostChanged {
                host_id: self.host_id.clone(),
            });
        }
        Ok(events)
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

        let forfeit_winner = match (&outcome, reason) {
            (RoundOutcome::Win { winner }, RoundEndReason::PlayerLeft) => Some(winner.clone()),
            _ => None,
        };
        let winner = forfeit_winner.or_else(|| {
            self.players
                .values()
                .find(|p| p.score >= self.rules.target_score)
                .map(|p| p.id.clone())
        });
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
            if reason == RoundEndReason::PlayerLeft && active.len() == 1 {
                return Ok(RoundOutcome::Win {
                    winner: active[0].clone(),
                });
            }
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
                participant_seat_expires_at_ms: None,
                spectator_expires_at_ms: None,
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

    fn transfer_host_if_current(&mut self, player_id: &PlayerId, events: &mut Vec<RoomEvent>) {
        if self.host_id.as_ref() == Some(player_id) {
            self.host_id = self.next_host_id();
            events.push(RoomEvent::HostChanged {
                host_id: self.host_id.clone(),
            });
        }
    }

    fn participant_count(&self) -> usize {
        self.players
            .values()
            .filter(|p| p.role == PlayerRole::Participant)
            .count()
    }

    fn match_in_progress(&self) -> bool {
        self.round > 0 && !matches!(self.phase, RoomPhase::Finished { .. })
    }

    fn finish_match_by_forfeit(&mut self) -> Vec<RoomEvent> {
        let winner = self.forfeit_winner_id();
        self.moves.clear();
        for player in self.players.values_mut() {
            player.ready = false;
        }
        self.phase = RoomPhase::Finished {
            winner: winner.clone(),
        };
        vec![RoomEvent::GameEnded { winner }]
    }

    fn forfeit_winner_id(&self) -> Option<PlayerId> {
        self.players
            .values()
            .find(|player| player.role == PlayerRole::Participant && player.connected)
            .or_else(|| {
                self.players
                    .values()
                    .find(|player| player.role == PlayerRole::Participant)
            })
            .map(|player| player.id.clone())
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
                participant_seat_expires_at_ms: None,
                spectator_expires_at_ms: None,
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
    fn duplicate_names_are_rejected_across_participants_and_spectators() {
        let host = Player::participant(PlayerId::new("host"), "Alex");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

        let spectator_err = room
            .apply(RoomCommand::Join {
                player: Player::spectator(PlayerId::new("watcher"), "alex"),
            })
            .unwrap_err();
        assert_eq!(
            spectator_err,
            CoreError::DuplicatePlayerName("alex".to_owned())
        );

        room.apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("mira"), "Mira"),
        })
        .unwrap();

        let participant_err = room
            .apply(RoomCommand::Join {
                player: Player::participant(PlayerId::new("guest"), "mira"),
            })
            .unwrap_err();
        assert_eq!(
            participant_err,
            CoreError::DuplicatePlayerName("mira".to_owned())
        );
    }

    #[test]
    fn duplicate_names_are_rejected_between_spectators() {
        let host = Player::participant(PlayerId::new("host"), "Host");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
        room.apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("watcher-one"), "Mira"),
        })
        .unwrap();

        let err = room
            .apply(RoomCommand::Join {
                player: Player::spectator(PlayerId::new("watcher-two"), "mira"),
            })
            .unwrap_err();

        assert_eq!(err, CoreError::DuplicatePlayerName("mira".to_owned()));
    }

    #[test]
    fn player_can_be_renamed_within_room() {
        let mut room = two_player_room();

        let events = room
            .apply(RoomCommand::RenamePlayer {
                player_id: PlayerId::new("alice"),
                name: "  Alicia  ".to_owned(),
            })
            .unwrap();
        let snapshot = room.snapshot();
        let alice = snapshot
            .players
            .iter()
            .find(|player| player.id == PlayerId::new("alice"))
            .unwrap();

        assert_eq!(
            events,
            vec![RoomEvent::PlayerRenamed {
                player_id: PlayerId::new("alice"),
                name: "Alicia".to_owned(),
            }]
        );
        assert_eq!(alice.name, "Alicia");
        assert_eq!(snapshot.scoreboard[0].name, "Alicia");
    }

    #[test]
    fn rename_rejects_duplicate_or_empty_name() {
        let mut room = two_player_room();

        let duplicate = room
            .apply(RoomCommand::RenamePlayer {
                player_id: PlayerId::new("alice"),
                name: " bob ".to_owned(),
            })
            .unwrap_err();
        let empty = room
            .apply(RoomCommand::RenamePlayer {
                player_id: PlayerId::new("alice"),
                name: "  ".to_owned(),
            })
            .unwrap_err();

        assert_eq!(duplicate, CoreError::DuplicatePlayerName("bob".to_owned()));
        assert_eq!(empty, CoreError::EmptyName);
    }

    #[test]
    fn current_host_can_update_race_target_before_match_starts() {
        let mut room = two_player_room();

        let events = room
            .apply(RoomCommand::UpdateMatchFormat {
                player_id: PlayerId::new("alice"),
                target_score: 3,
            })
            .unwrap();

        assert_eq!(room.snapshot().rules.target_score, 3);
        assert_eq!(
            events,
            vec![RoomEvent::MatchFormatChanged { target_score: 3 }]
        );
    }

    #[test]
    fn race_target_update_rejects_unsupported_targets() {
        let mut room = two_player_room();

        let err = room
            .apply(RoomCommand::UpdateMatchFormat {
                player_id: PlayerId::new("alice"),
                target_score: 5,
            })
            .unwrap_err();

        assert_eq!(
            err,
            CoreError::InvalidRules("target_score must be one of 1, 2, or 3".to_owned())
        );
    }

    #[test]
    fn updating_race_target_clears_ready_players() {
        let host = Player::participant(PlayerId::new("alice"), "Alice");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

        room.apply(RoomCommand::SetReady {
            player_id: PlayerId::new("alice"),
            ready: true,
            now_ms: 1_000,
        })
        .unwrap();
        let events = room
            .apply(RoomCommand::UpdateMatchFormat {
                player_id: PlayerId::new("alice"),
                target_score: 3,
            })
            .unwrap();
        let alice = room
            .snapshot()
            .players
            .into_iter()
            .find(|player| player.id == PlayerId::new("alice"))
            .unwrap();

        assert!(!alice.ready);
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::ReadyChanged { player_id, ready }
                if player_id == &PlayerId::new("alice") && !ready
        )));
    }

    #[test]
    fn non_host_cannot_update_race_target() {
        let mut room = two_player_room();

        let err = room
            .apply(RoomCommand::UpdateMatchFormat {
                player_id: PlayerId::new("bob"),
                target_score: 3,
            })
            .unwrap_err();

        assert_eq!(err, CoreError::HostOnly);
    }

    #[test]
    fn transferred_host_can_update_race_target() {
        let mut room = two_player_room();

        room.apply(RoomCommand::Leave {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
        room.apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("bob"),
            target_score: 3,
        })
        .unwrap();

        assert_eq!(room.snapshot().host_id, Some(PlayerId::new("bob")));
        assert_eq!(room.snapshot().rules.target_score, 3);
    }

    #[test]
    fn race_target_cannot_change_during_unfinished_match() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1_000);

        let err = room
            .apply(RoomCommand::UpdateMatchFormat {
                player_id: PlayerId::new("alice"),
                target_score: 3,
            })
            .unwrap_err();

        assert_eq!(err, CoreError::MatchInProgress);
    }

    #[test]
    fn setting_ready_to_existing_value_is_noop() {
        let host = Player::participant(PlayerId::new("alice"), "Alice");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();

        room.apply(RoomCommand::SetReady {
            player_id: PlayerId::new("alice"),
            ready: true,
            now_ms: 1_000,
        })
        .unwrap();
        let events = room
            .apply(RoomCommand::SetReady {
                player_id: PlayerId::new("alice"),
                ready: true,
                now_ms: 1_001,
            })
            .unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn setting_existing_spectator_role_is_noop() {
        let host = Player::participant(PlayerId::new("alice"), "Alice");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
        room.apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("mira"), "Mira"),
        })
        .unwrap();

        let events = room
            .apply(RoomCommand::SetSpectator {
                player_id: PlayerId::new("mira"),
                spectator: true,
            })
            .unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn disconnected_player_cannot_switch_role() {
        let host = Player::participant(PlayerId::new("alice"), "Alice");
        let mut room =
            GameRoom::new(RoomId::new("room"), "room", GameRules::default(), host).unwrap();
        room.apply(RoomCommand::Disconnect {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();

        let err = room
            .apply(RoomCommand::SetSpectator {
                player_id: PlayerId::new("alice"),
                spectator: true,
            })
            .unwrap_err();

        assert_eq!(err, CoreError::PlayerDisconnected);
    }

    #[test]
    fn host_can_update_race_target_after_match_finishes() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1_000);
        alice_wins_round(&mut room, 2_000);

        room.apply(RoomCommand::UpdateMatchFormat {
            player_id: PlayerId::new("alice"),
            target_score: 3,
        })
        .unwrap();

        assert_eq!(room.snapshot().rules.target_score, 3);
    }

    #[test]
    fn finished_match_rejects_ready_and_moves_until_host_resets() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);
        alice_wins_round(&mut room, 2000);

        let ready_err = room
            .apply(RoomCommand::SetReady {
                player_id: PlayerId::new("alice"),
                ready: true,
                now_ms: 3000,
            })
            .unwrap_err();
        assert_eq!(ready_err, CoreError::RoomFinished);

        let move_err = room
            .apply(RoomCommand::SubmitMove {
                player_id: PlayerId::new("alice"),
                mv: Move::Rock,
                now_ms: 3001,
            })
            .unwrap_err();
        assert_eq!(move_err, CoreError::RoundNotActive);

        room.apply(RoomCommand::StartNextMatch {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
        ready(&mut room, "alice", 4000);
        ready(&mut room, "bob", 4000);

        assert!(matches!(room.phase(), RoomPhase::InRound { round: 1, .. }));
    }

    #[test]
    fn submit_move_before_deadline_is_accepted() {
        let mut room = two_player_room();
        ready(&mut room, "alice", 1000);
        ready(&mut room, "bob", 1000);
        let deadline_ms = match room.phase() {
            RoomPhase::InRound { deadline_ms, .. } => *deadline_ms,
            _ => panic!("round should be active"),
        };

        let events = room
            .apply(RoomCommand::SubmitMove {
                player_id: PlayerId::new("alice"),
                mv: Move::Rock,
                now_ms: deadline_ms - 1,
            })
            .unwrap();

        assert_eq!(room.moves.get(&PlayerId::new("alice")), Some(&Move::Rock));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::MoveAccepted { player_id } if player_id == &PlayerId::new("alice")
        )));
    }

    #[test]
    fn submit_move_at_or_after_deadline_is_rejected() {
        let mut room = two_player_room();
        ready(&mut room, "alice", 1000);
        ready(&mut room, "bob", 1000);
        let (round, deadline_ms) = match room.phase() {
            RoomPhase::InRound { round, deadline_ms } => (*round, *deadline_ms),
            _ => panic!("round should be active"),
        };

        let err = room
            .apply(RoomCommand::SubmitMove {
                player_id: PlayerId::new("alice"),
                mv: Move::Rock,
                now_ms: deadline_ms,
            })
            .unwrap_err();

        assert_eq!(err, CoreError::RoundExpired);
        assert!(!room.moves.contains_key(&PlayerId::new("alice")));
        room.apply(RoomCommand::TimeoutRound {
            round,
            now_ms: deadline_ms,
        })
        .unwrap();
        assert!(matches!(room.phase(), RoomPhase::Lobby));
    }

    #[test]
    fn participant_seat_expiry_demotes_disconnected_player_to_spectator() {
        let mut room = two_player_room();

        room.apply(RoomCommand::Disconnect {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
        let events = room
            .apply(RoomCommand::ExpireParticipantSeat {
                player_id: PlayerId::new("alice"),
            })
            .unwrap();
        let snapshot = room.snapshot();
        let alice = snapshot
            .players
            .iter()
            .find(|player| player.id == PlayerId::new("alice"))
            .unwrap();

        assert_eq!(alice.role, PlayerRole::Spectator);
        assert!(!alice.connected);
        assert_eq!(snapshot.host_id, Some(PlayerId::new("bob")));
        assert!(!snapshot
            .scoreboard
            .iter()
            .any(|score| score.player_id == PlayerId::new("alice")));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::RoleChanged { player_id, role }
                if player_id == &PlayerId::new("alice") && role == &PlayerRole::Spectator
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::HostChanged { host_id } if host_id == &Some(PlayerId::new("bob"))
        )));
    }

    #[test]
    fn participant_seat_expiry_is_noop_after_reconnect() {
        let mut room = two_player_room();

        room.apply(RoomCommand::Disconnect {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
        room.apply(RoomCommand::Reconnect {
            player_id: PlayerId::new("alice"),
        })
        .unwrap();
        let events = room
            .apply(RoomCommand::ExpireParticipantSeat {
                player_id: PlayerId::new("alice"),
            })
            .unwrap();
        let snapshot = room.snapshot();
        let alice = snapshot
            .players
            .iter()
            .find(|player| player.id == PlayerId::new("alice"))
            .unwrap();

        assert!(events.is_empty());
        assert_eq!(alice.role, PlayerRole::Participant);
        assert!(alice.connected);
        assert_eq!(snapshot.host_id, Some(PlayerId::new("alice")));
    }

    #[test]
    fn race_to_two_finishes_when_a_player_reaches_two_points() {
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
    fn role_switching_is_rejected_between_unfinished_rounds() {
        let mut room = two_player_room();
        room.apply(RoomCommand::Join {
            player: Player::spectator(PlayerId::new("mira"), "Mira"),
        })
        .unwrap();
        alice_wins_round(&mut room, 1000);

        let participant_err = room
            .apply(RoomCommand::SetSpectator {
                player_id: PlayerId::new("alice"),
                spectator: true,
            })
            .unwrap_err();
        let spectator_err = room
            .apply(RoomCommand::SetSpectator {
                player_id: PlayerId::new("mira"),
                spectator: false,
            })
            .unwrap_err();

        assert_eq!(participant_err, CoreError::MatchInProgress);
        assert_eq!(spectator_err, CoreError::MatchInProgress);
    }

    #[test]
    fn participant_join_is_rejected_between_unfinished_rounds() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);

        let err = room
            .apply(RoomCommand::Join {
                player: Player::participant(PlayerId::new("carol"), "Carol"),
            })
            .unwrap_err();

        assert_eq!(err, CoreError::MatchInProgress);
    }

    #[test]
    fn participant_leave_between_rounds_finishes_match_by_forfeit() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);

        let events = room
            .apply(RoomCommand::Leave {
                player_id: PlayerId::new("bob"),
            })
            .unwrap();

        assert!(matches!(
            room.phase(),
            RoomPhase::Finished {
                winner: Some(player_id)
            } if player_id == &PlayerId::new("alice")
        ));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::PlayerLeft { player_id } if player_id == &PlayerId::new("bob")
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::GameEnded { winner } if winner == &Some(PlayerId::new("alice"))
        )));
        assert_eq!(
            room.snapshot().scoreboard[0].player_id,
            PlayerId::new("alice")
        );
    }

    #[test]
    fn participant_disconnect_during_round_finishes_match_by_forfeit() {
        let mut room = two_player_room();
        ready(&mut room, "alice", 1000);
        ready(&mut room, "bob", 1000);

        let events = room
            .apply(RoomCommand::Disconnect {
                player_id: PlayerId::new("bob"),
            })
            .unwrap();

        assert!(matches!(
            room.phase(),
            RoomPhase::Finished {
                winner: Some(player_id)
            } if player_id == &PlayerId::new("alice")
        ));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::RoundResolved { result }
                if result.reason == RoundEndReason::PlayerLeft
                    && matches!(&result.outcome, RoundOutcome::Win { winner } if winner == &PlayerId::new("alice"))
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::GameEnded { winner } if winner == &Some(PlayerId::new("alice"))
        )));
        assert_eq!(room.snapshot().scoreboard[0].score, 1);
    }

    #[test]
    fn host_disconnect_during_round_transfers_host_and_forfeits_match() {
        let mut room = two_player_room();
        ready(&mut room, "alice", 1000);
        ready(&mut room, "bob", 1000);

        let events = room
            .apply(RoomCommand::Disconnect {
                player_id: PlayerId::new("alice"),
            })
            .unwrap();
        let snapshot = room.snapshot();

        assert!(matches!(
            room.phase(),
            RoomPhase::Finished {
                winner: Some(player_id)
            } if player_id == &PlayerId::new("bob")
        ));
        assert_eq!(snapshot.host_id, Some(PlayerId::new("bob")));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::HostChanged { host_id } if host_id == &Some(PlayerId::new("bob"))
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::RoundResolved { result }
                if result.reason == RoundEndReason::PlayerLeft
                    && matches!(&result.outcome, RoundOutcome::Win { winner } if winner == &PlayerId::new("bob"))
        )));
        assert!(snapshot
            .scoreboard
            .iter()
            .any(|score| score.player_id == PlayerId::new("bob") && score.score == 1));
    }

    #[test]
    fn host_disconnect_between_rounds_transfers_host_and_forfeits_match() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);

        let events = room
            .apply(RoomCommand::Disconnect {
                player_id: PlayerId::new("alice"),
            })
            .unwrap();
        let snapshot = room.snapshot();

        assert!(matches!(
            room.phase(),
            RoomPhase::Finished {
                winner: Some(player_id)
            } if player_id == &PlayerId::new("bob")
        ));
        assert_eq!(snapshot.host_id, Some(PlayerId::new("bob")));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::HostChanged { host_id } if host_id == &Some(PlayerId::new("bob"))
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::GameEnded { winner } if winner == &Some(PlayerId::new("bob"))
        )));
    }

    #[test]
    fn spectator_join_is_allowed_during_unfinished_match() {
        let mut room = two_player_room();
        alice_wins_round(&mut room, 1000);

        let events = room
            .apply(RoomCommand::Join {
                player: Player::spectator(PlayerId::new("mira"), "Mira"),
            })
            .unwrap();

        assert!(events.iter().any(|event| matches!(
            event,
            RoomEvent::PlayerJoined { player_id, role, .. }
                if player_id == &PlayerId::new("mira") && role == &PlayerRole::Spectator
        )));
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
