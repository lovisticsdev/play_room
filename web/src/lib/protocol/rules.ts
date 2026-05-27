import type { GameKind, GameRules, RoomPhase } from './types';

export function defaultRules(game: GameKind = 'rock_paper_scissors'): GameRules {
  return {
    game,
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
