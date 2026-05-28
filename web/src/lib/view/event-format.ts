import type { PlayerId, RoomEvent, RoomSnapshot, RoundOutcome } from '../protocol/types';

function nameForPlayer(room: RoomSnapshot | null, playerId: PlayerId | null): string {
  if (!playerId) return 'No one';
  return room?.players.find((player) => player.id === playerId)?.name ?? playerId;
}

function outcomeText(room: RoomSnapshot | null, outcome: RoundOutcome): string {
  if (outcome === 'draw') return 'Round resolved as a draw';
  if (outcome === 'no_contest') return 'Round resolved with no contest';
  if ('win' in outcome) return `${nameForPlayer(room, outcome.win.winner)} wins the round`;
  return `${nameForPlayer(room, outcome.timeout_win.winner)} wins by timeout`;
}

export function formatRoomEvent(event: RoomEvent, room: RoomSnapshot | null): string {
  switch (event.event) {
    case 'player_joined':
      return `${event.name} joined as ${event.role}`;
    case 'player_left':
      return `${nameForPlayer(room, event.player_id)} left the room`;
    case 'player_disconnected':
      return `${nameForPlayer(room, event.player_id)} disconnected`;
    case 'player_reconnected':
      return `${nameForPlayer(room, event.player_id)} reconnected`;
    case 'ready_changed':
      return `${nameForPlayer(room, event.player_id)} is ${event.ready ? 'ready' : 'not ready'}`;
    case 'role_changed':
      return `${nameForPlayer(room, event.player_id)} switched to ${event.role}`;
    case 'round_started':
      return `Round ${event.round} started`;
    case 'move_accepted':
      return `${nameForPlayer(room, event.player_id)} locked a move`;
    case 'round_resolved':
      return outcomeText(room, event.result.outcome);
    case 'game_ended':
      return event.winner ? `${nameForPlayer(room, event.winner)} won the match` : 'Match ended';
    case 'match_reset':
      return `${nameForPlayer(room, event.requested_by)} started the next match`;
    case 'host_changed':
      return `${nameForPlayer(room, event.host_id)} is now host`;
  }
}
