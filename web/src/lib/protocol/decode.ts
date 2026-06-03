import type {
  ErrorCode,
  GameKind,
  GameRules,
  Move,
  PlayerRole,
  PlayerScore,
  PlayerView,
  RoomEvent,
  RoomPhase,
  RoomSnapshot,
  RoomSummary,
  RoundEndReason,
  RoundOutcome,
  RoundResult,
  ServerEvent,
  ServerMessage,
  ServerResult,
} from './types';

const GAME_KINDS = new Set<GameKind>(['rock_paper_scissors', 'rock_paper_scissors_lizard_spock']);
const MOVES = new Set<Move>(['rock', 'paper', 'scissors', 'lizard', 'spock']);
const PLAYER_ROLES = new Set<PlayerRole>(['participant', 'spectator']);
const ERROR_CODES = new Set<ErrorCode>([
  'invalid_request',
  'room_not_found',
  'room_name_exists',
  'player_name_exists',
  'room_full',
  'not_in_room',
  'match_not_finished',
  'host_only',
  'invalid_action',
]);
const ROUND_REASONS = new Set<RoundEndReason>(['all_moves_submitted', 'timeout', 'player_left']);

export class ProtocolDecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ProtocolDecodeError';
  }
}

export function decodeServerMessage(raw: unknown): ServerMessage {
  let parsed: unknown;
  try {
    parsed = JSON.parse(String(raw));
  } catch {
    throw new ProtocolDecodeError('Server message is not valid JSON.');
  }

  if (!isServerMessage(parsed)) {
    throw new ProtocolDecodeError('Server message does not match the Play Room protocol.');
  }

  return parsed;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === 'string';
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

function isNonNegativeSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function isPositiveSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}

function isStringOrNull(value: unknown): value is string | null {
  return value === null || isString(value);
}

function isOptionalNonNegativeSafeIntegerOrNull(value: unknown): value is number | null | undefined {
  return value === undefined || value === null || isNonNegativeSafeInteger(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(isString);
}

function isGameKind(value: unknown): value is GameKind {
  return isString(value) && GAME_KINDS.has(value as GameKind);
}

function isMove(value: unknown): value is Move {
  return isString(value) && MOVES.has(value as Move);
}

function isPlayerRole(value: unknown): value is PlayerRole {
  return isString(value) && PLAYER_ROLES.has(value as PlayerRole);
}

function isErrorCode(value: unknown): value is ErrorCode {
  return isString(value) && ERROR_CODES.has(value as ErrorCode);
}

function isRoundEndReason(value: unknown): value is RoundEndReason {
  return isString(value) && ROUND_REASONS.has(value as RoundEndReason);
}

function isServerMessage(value: unknown): value is ServerMessage {
  if (!isRecord(value)) return false;

  if (value.kind === 'response') {
    return isNonNegativeSafeInteger(value.request_id) && isServerResult(value.result);
  }

  if (value.kind === 'event') {
    return isServerEvent(value.event);
  }

  return false;
}

function isServerResult(value: unknown): value is ServerResult {
  if (!isRecord(value) || !isString(value.status)) return false;

  switch (value.status) {
    case 'ok':
    case 'pong':
      return true;
    case 'error':
      return isString(value.message)
        && (value.code === undefined || value.code === null || isErrorCode(value.code))
        && (value.suggestions === undefined || isStringArray(value.suggestions));
    case 'welcome':
      return isString(value.player_id)
        && isString(value.reconnect_token)
        && isPositiveSafeInteger(value.protocol_version)
        && isBoolean(value.reconnected)
        && isBoolean(value.stale_token_replaced)
        && isBoolean(value.room_restored);
    case 'room_list':
      return Array.isArray(value.rooms) && value.rooms.every(isRoomSummary);
    case 'room_snapshot':
      return isRoomSnapshot(value.room);
    default:
      return false;
  }
}

function isServerEvent(value: unknown): value is ServerEvent {
  if (!isRecord(value) || !isString(value.type)) return false;

  switch (value.type) {
    case 'notice':
      return isString(value.message);
    case 'room_event':
      return isString(value.room_id) && isRoomEvent(value.event);
    case 'room_snapshot':
      return isRoomSnapshot(value.room);
    default:
      return false;
  }
}

function isRoomEvent(value: unknown): value is RoomEvent {
  if (!isRecord(value) || !isString(value.event)) return false;

  switch (value.event) {
    case 'player_joined':
      return isString(value.player_id) && isString(value.name) && isPlayerRole(value.role);
    case 'player_left':
    case 'player_disconnected':
    case 'player_reconnected':
    case 'move_accepted':
      return isString(value.player_id);
    case 'ready_changed':
      return isString(value.player_id) && isBoolean(value.ready);
    case 'role_changed':
      return isString(value.player_id) && isPlayerRole(value.role);
    case 'round_started':
      return isPositiveSafeInteger(value.round) && isNonNegativeSafeInteger(value.deadline_ms);
    case 'round_resolved':
      return isRoundResult(value.result);
    case 'game_ended':
      return isStringOrNull(value.winner);
    case 'match_reset':
      return isString(value.requested_by);
    case 'host_changed':
      return isStringOrNull(value.host_id);
    default:
      return false;
  }
}

function isGameRules(value: unknown): value is GameRules {
  return isRecord(value)
    && isGameKind(value.game)
    && value.min_players === 2
    && value.max_players === 2
    && isPositiveSafeInteger(value.target_score)
    && isPositiveSafeInteger(value.round_seconds)
    && isBoolean(value.allow_spectators);
}

function isRoomPhase(value: unknown): value is RoomPhase {
  if (!isRecord(value) || !isString(value.phase)) return false;

  switch (value.phase) {
    case 'lobby':
      return true;
    case 'in_round':
      return isPositiveSafeInteger(value.round) && isNonNegativeSafeInteger(value.deadline_ms);
    case 'finished':
      return isStringOrNull(value.winner);
    default:
      return false;
  }
}

function isPlayerView(value: unknown): value is PlayerView {
  return isRecord(value)
    && isString(value.id)
    && isString(value.name)
    && isPlayerRole(value.role)
    && isBoolean(value.ready)
    && isBoolean(value.connected)
    && isNonNegativeSafeInteger(value.score)
    && isOptionalNonNegativeSafeIntegerOrNull(value.participant_seat_expires_at_ms)
    && isOptionalNonNegativeSafeIntegerOrNull(value.spectator_expires_at_ms);
}

function isPlayerScore(value: unknown): value is PlayerScore {
  return isRecord(value)
    && isString(value.player_id)
    && isString(value.name)
    && isNonNegativeSafeInteger(value.score);
}

function isRoomSummary(value: unknown): value is RoomSummary {
  return isRecord(value)
    && isString(value.id)
    && isString(value.name)
    && isRoomPhase(value.phase)
    && isNonNegativeSafeInteger(value.players)
    && isNonNegativeSafeInteger(value.spectators)
    && isPositiveSafeInteger(value.max_players)
    && isGameKind(value.game)
    && isPositiveSafeInteger(value.target_score);
}

function isRoomSnapshot(value: unknown): value is RoomSnapshot {
  return isRecord(value)
    && isString(value.id)
    && isString(value.name)
    && isStringOrNull(value.host_id)
    && isRoomPhase(value.phase)
    && isGameRules(value.rules)
    && isNonNegativeSafeInteger(value.round)
    && Array.isArray(value.players)
    && value.players.every(isPlayerView)
    && Array.isArray(value.scoreboard)
    && value.scoreboard.every(isPlayerScore);
}

function isRoundOutcome(value: unknown): value is RoundOutcome {
  if (value === 'draw' || value === 'no_contest') return true;
  if (!isRecord(value)) return false;

  if (isRecord(value.win)) {
    return isString(value.win.winner);
  }

  if (isRecord(value.timeout_win)) {
    return isString(value.timeout_win.winner);
  }

  return false;
}

function isMoveOrNull(value: unknown): value is Move | null {
  return value === null || isMove(value);
}

function isSubmittedMoves(value: unknown): value is Record<string, Move | null> {
  return isRecord(value) && Object.values(value).every(isMoveOrNull);
}

function isRoundResult(value: unknown): value is RoundResult {
  return isRecord(value)
    && isPositiveSafeInteger(value.round)
    && isRoundEndReason(value.reason)
    && isSubmittedMoves(value.submitted)
    && isRoundOutcome(value.outcome)
    && Array.isArray(value.scores)
    && value.scores.every(isPlayerScore);
}