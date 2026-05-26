export type Id = string;
export type PlayerId = Id;
export type RoomId = Id;
export type SessionToken = Id;

export type GameKind = 'rock_paper_scissors' | 'rock_paper_scissors_lizard_spock';
export type Move = 'rock' | 'paper' | 'scissors' | 'lizard' | 'spock';
export type PlayerRole = 'participant' | 'spectator';

export interface GameRules {
  game: GameKind;
  min_players: number;
  max_players: number;
  target_score: number;
  round_seconds: number;
  allow_spectators: boolean;
}

export type RoomPhase =
  | { phase: 'lobby' }
  | { phase: 'in_round'; round: number; deadline_ms: number }
  | { phase: 'finished' };

export interface PlayerView {
  id: PlayerId;
  name: string;
  role: PlayerRole;
  ready: boolean;
  connected: boolean;
  score: number;
}

export interface PlayerScore {
  player_id: PlayerId;
  name: string;
  score: number;
}

export interface RoomSummary {
  id: RoomId;
  name: string;
  phase: RoomPhase;
  players: number;
  spectators: number;
  max_players: number;
}

export interface RoomSnapshot {
  id: RoomId;
  name: string;
  host_id: PlayerId | null;
  phase: RoomPhase;
  rules: GameRules;
  round: number;
  players: PlayerView[];
  scoreboard: PlayerScore[];
}

export type RoundOutcome =
  | 'draw'
  | 'no_contest'
  | { win: { winner: PlayerId } }
  | { timeout_win: { winner: PlayerId } };

export type RoundEndReason = 'all_moves_submitted' | 'timeout' | 'player_left';

export interface RoundResult {
  round: number;
  reason: RoundEndReason;
  submitted: Record<PlayerId, Move | null>;
  outcome: RoundOutcome;
  scores: PlayerScore[];
}

export type RoomEvent =
  | { event: 'player_joined'; player_id: PlayerId; name: string; role: PlayerRole }
  | { event: 'player_left'; player_id: PlayerId }
  | { event: 'player_disconnected'; player_id: PlayerId }
  | { event: 'player_reconnected'; player_id: PlayerId }
  | { event: 'ready_changed'; player_id: PlayerId; ready: boolean }
  | { event: 'role_changed'; player_id: PlayerId; role: PlayerRole }
  | { event: 'round_started'; round: number; deadline_ms: number }
  | { event: 'move_accepted'; player_id: PlayerId; mv: Move }
  | { event: 'round_resolved'; result: RoundResult }
  | { event: 'game_ended'; winner: PlayerId | null }
  | { event: 'host_changed'; host_id: PlayerId | null };

export type ClientRequest =
  | { type: 'connect'; name: string; reconnect_token: SessionToken | null }
  | { type: 'list_rooms' }
  | { type: 'create_room'; name: string; rules: GameRules | null }
  | { type: 'join_room'; room_id: RoomId }
  | { type: 'spectate_room'; room_id: RoomId }
  | { type: 'leave_room' }
  | { type: 'set_ready'; ready: boolean }
  | { type: 'set_spectator'; spectator: boolean }
  | { type: 'submit_move'; mv: Move }
  | { type: 'ping' };

export interface ClientEnvelope {
  request_id: number;
  request: ClientRequest;
}

export type ServerResult =
  | { status: 'ok' }
  | { status: 'error'; message: string }
  | { status: 'welcome'; player_id: PlayerId; reconnect_token: SessionToken; protocol_version: number }
  | { status: 'room_list'; rooms: RoomSummary[] }
  | { status: 'room_snapshot'; room: RoomSnapshot }
  | { status: 'pong' };

export type ServerEvent =
  | { type: 'notice'; message: string }
  | { type: 'room_event'; room_id: RoomId; event: RoomEvent }
  | { type: 'room_snapshot'; room: RoomSnapshot };

export type ServerMessage =
  | { kind: 'response'; request_id: number; result: ServerResult }
  | { kind: 'event'; event: ServerEvent };

export interface WelcomeState {
  player_id: PlayerId;
  reconnect_token: SessionToken;
  protocol_version: number;
}

export function defaultRules(): GameRules {
  return {
    game: 'rock_paper_scissors',
    min_players: 2,
    max_players: 2,
    target_score: 3,
    round_seconds: 15,
    allow_spectators: true,
  };
}

export function phaseLabel(phase: RoomPhase): string {
  if (phase.phase === 'lobby') return 'Lobby';
  if (phase.phase === 'finished') return 'Finished';
  return `Round ${phase.round}`;
}

export function isInRound(phase: RoomPhase): phase is { phase: 'in_round'; round: number; deadline_ms: number } {
  return phase.phase === 'in_round';
}

export function roleLabel(role: PlayerRole): string {
  return role === 'participant' ? 'Participant' : 'Spectator';
}
