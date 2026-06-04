import type { PlayerId, PlayerRole, RoomEvent, RoomSnapshot, RoundResult } from '../protocol/types';

function nameForPlayer(room: RoomSnapshot | null, playerId: PlayerId | null): string {
  if (!playerId) return 'No one';
  return room?.players.find((player) => player.id === playerId)?.name ?? playerId;
}

function outcomeText(room: RoomSnapshot | null, result: RoundResult): string {
  const { outcome } = result;
  if (outcome === 'draw') return 'Round resolved as a draw';
  if (outcome === 'no_contest') return 'Round resolved with no contest';
  if ('win' in outcome) {
    return result.reason === 'player_left'
      ? `${nameForPlayer(room, outcome.win.winner)} wins by forfeit`
      : `${nameForPlayer(room, outcome.win.winner)} wins the round`;
  }
  return `${nameForPlayer(room, outcome.timeout_win.winner)} wins by timeout`;
}

function roleText(role: PlayerRole): string {
  return role === 'participant' ? 'player' : 'spectator';
}

export function formatRoomEvent(event: RoomEvent, room: RoomSnapshot | null): string {
  switch (event.event) {
    case 'player_joined':
      return `${event.name} joined as ${roleText(event.role)}`;
    case 'player_left':
      return `${nameForPlayer(room, event.player_id)} left the room`;
    case 'player_disconnected': {
      const previous = room?.players.find((player) => player.id === event.player_id);
      const grace = previous?.role === 'spectator' ? 'name reserved for 60s' : 'participant seat reserved for 30s';
      return `${nameForPlayer(room, event.player_id)} disconnected; ${grace}`;
    }
    case 'player_reconnected':
      return `${nameForPlayer(room, event.player_id)} reconnected`;
    case 'ready_changed':
      return `${nameForPlayer(room, event.player_id)} is ${event.ready ? 'ready' : 'not ready'}`;
    case 'role_changed': {
      const previous = room?.players.find((player) => player.id === event.player_id);
      if (event.role === 'spectator' && previous?.role === 'participant' && !previous.connected) {
        return `${previous.name}'s reserved seat expired; now watching`;
      }
      return `${nameForPlayer(room, event.player_id)} switched to ${roleText(event.role)}`;
    }
    case 'round_started':
      return `Round ${event.round} started`;
    case 'move_accepted':
      return `${nameForPlayer(room, event.player_id)} locked a move`;
    case 'round_resolved':
      return outcomeText(room, event.result);
    case 'game_ended':
      return event.winner ? `${nameForPlayer(room, event.winner)} won the match` : 'Match ended';
    case 'match_reset':
      return `${nameForPlayer(room, event.requested_by)} started the next match`;
    case 'host_changed':
      return `${nameForPlayer(room, event.host_id)} is now host`;
  }
}
