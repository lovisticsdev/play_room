import type { PlayerId, PlayerView, RoomPhase, RoomSnapshot, RoomSummary, RoundOutcome } from '../protocol/types';
import { gameLabel, isFinished, isInRound, raceToLabel } from '../protocol/rules';

export type RoomAction = 'current' | 'join' | 'watch';

export function participants(room: RoomSnapshot | null): PlayerView[] {
  return room?.players.filter((player) => player.role === 'participant') ?? [];
}

export function spectators(room: RoomSnapshot | null): PlayerView[] {
  return room?.players.filter((player) => player.role === 'spectator') ?? [];
}

export function localPlayer(room: RoomSnapshot | null, playerId: PlayerId | null): PlayerView | null {
  if (!room || !playerId) return null;
  return room.players.find((player) => player.id === playerId) ?? null;
}

export function opponent(room: RoomSnapshot | null, playerId: PlayerId | null): PlayerView | null {
  return participants(room).find((player) => player.id !== playerId) ?? null;
}

export function connectedParticipantCount(room: RoomSnapshot | null): number {
  return participants(room).filter((player) => player.connected).length;
}

export function getRoomAction(room: RoomSummary, currentRoomId: string | null): RoomAction {
  if (room.id === currentRoomId) return 'current';
  if (room.players < room.max_players && !isInRound(room.phase)) return 'join';
  return 'watch';
}

export function roomPhaseTag(phase: RoomPhase): string {
  if (phase.phase === 'lobby') return 'Lobby';
  if (phase.phase === 'finished') return 'Finished';
  return `Round ${phase.round} · In Progress`;
}

export function roomSummaryMeta(room: RoomSummary): string {
  return `${raceToLabel(room.target_score)} · ${gameLabel(room.game)}`;
}

export function roundCountdownDeadline(phase: RoomPhase): number | null {
  return isInRound(phase) ? phase.deadline_ms : null;
}

export function winnerId(outcome: RoundOutcome): PlayerId | null {
  if (outcome === 'draw' || outcome === 'no_contest') return null;
  if ('win' in outcome) return outcome.win.winner;
  return outcome.timeout_win.winner;
}

export function matchWinnerId(room: RoomSnapshot | null): PlayerId | null {
  return room && isFinished(room.phase) ? room.phase.winner : null;
}

export function playerName(room: RoomSnapshot | null, playerId: PlayerId | null): string {
  if (!playerId) return 'No one';
  return room?.players.find((player) => player.id === playerId)?.name ?? playerId;
}
