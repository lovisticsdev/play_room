import { PROTOCOL_VALUES } from './generated';
import type { GameKind, GameRules, Move, PlayerId, RoomPhase } from './types';

export const RPS_MOVES: Move[] = [...PROTOCOL_VALUES.rps_moves];
export const RPSLS_MOVES: Move[] = [...PROTOCOL_VALUES.moves];

export const RACE_TARGETS = [...PROTOCOL_VALUES.supported_target_scores] as const;
export type RaceTarget = (typeof RACE_TARGETS)[number];

export function defaultRules(game: GameKind = 'rock_paper_scissors_lizard_spock', targetScore: RaceTarget = 2): GameRules {
  return {
    game,
    min_players: 2,
    max_players: 2,
    target_score: targetScore,
    round_seconds: 15,
    allow_spectators: true,
  };
}

export function raceToLabel(targetScore: number): string {
  return `Race to ${targetScore}`;
}

export function roundLabel(round: number, phase: RoomPhase): string {
  const currentRound = phase.phase === 'lobby' ? round + 1 : phase.phase === 'in_round' ? phase.round : round;

  return `Round ${Math.max(1, currentRound)}`;
}

export function gameLabel(game: GameKind): string {
  return game === 'rock_paper_scissors' ? 'RPS' : 'RPSLS';
}

export function phaseLabel(phase: RoomPhase): string {
  if (phase.phase === 'lobby') return 'Lobby';
  if (phase.phase === 'finished') return 'Finished';
  return `Round ${phase.round}`;
}

export function phaseTone(phase: RoomPhase): 'muted' | 'active' | 'done' {
  if (phase.phase === 'lobby') return 'muted';
  if (phase.phase === 'finished') return 'done';
  return 'active';
}

export function isInRound(phase: RoomPhase): phase is { phase: 'in_round'; round: number; deadline_ms: number } {
  return phase.phase === 'in_round';
}

export function isFinished(phase: RoomPhase): phase is { phase: 'finished'; winner: PlayerId | null } {
  return phase.phase === 'finished';
}

export function movesForGame(game: GameKind): Move[] {
  return game === 'rock_paper_scissors' ? RPS_MOVES : RPSLS_MOVES;
}

export function moveLabel(move: Move): string {
  const labels: Record<Move, string> = {
    rock: 'Rock',
    paper: 'Paper',
    scissors: 'Scissors',
    lizard: 'Lizard',
    spock: 'Spock',
  };

  return labels[move];
}

export function moveSymbol(move: Move): string {
  const symbols: Record<Move, string> = {
    rock: '✊',
    paper: '✋',
    scissors: '✂',
    lizard: '🦎',
    spock: '🖖',
  };

  return symbols[move];
}
