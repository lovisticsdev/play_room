import type { ClientRequest, GameRules, Move, RoomId, SessionToken } from './types';
import { defaultRules } from './rules';

function normalizeOptional(value: string | null | undefined): SessionToken | null {
  const trimmed = value?.trim() ?? '';
  return trimmed.length > 0 ? trimmed : null;
}

export function connectRequest(name: string, reconnectToken: string | null = null): ClientRequest {
  return {
    type: 'connect',
    name: name.trim(),
    reconnect_token: normalizeOptional(reconnectToken),
  };
}

export function reconnectRequest(reconnectToken: string): ClientRequest {
  return {
    type: 'connect',
    name: '',
    reconnect_token: reconnectToken.trim(),
  };
}

export function listRoomsRequest(): ClientRequest {
  return { type: 'list_rooms' };
}

export function createRoomRequest(name: string, rules: GameRules | null = defaultRules()): ClientRequest {
  return { type: 'create_room', name: name.trim(), rules };
}

export function joinRoomRequest(roomId: RoomId): ClientRequest {
  return { type: 'join_room', room_id: roomId.trim() };
}

export function spectateRoomRequest(roomId: RoomId): ClientRequest {
  return { type: 'spectate_room', room_id: roomId.trim() };
}

export function leaveRoomRequest(): ClientRequest {
  return { type: 'leave_room' };
}

export function startNextMatchRequest(): ClientRequest {
  return { type: 'start_next_match' };
}

export function submitMoveRequest(mv: Move): ClientRequest {
  return { type: 'submit_move', mv };
}

export function setReadyRequest(ready: boolean): ClientRequest {
  return { type: 'set_ready', ready };
}

export function setSpectatorRequest(spectator: boolean): ClientRequest {
  return { type: 'set_spectator', spectator };
}

export function pingRequest(): ClientRequest {
  return { type: 'ping' };
}