import type { ClientRequest, GameRules, Move, RoomId, SessionToken } from './protocol';
import { defaultRules } from './protocol';

export function connectRequest(name: string, reconnectToken: string): ClientRequest {
  return {
    type: 'connect',
    name: name.trim(),
    reconnect_token: normalizeOptional(reconnectToken),
  };
}

export function createRoomRequest(name: string, rules: GameRules = defaultRules()): ClientRequest {
  return {
    type: 'create_room',
    name: name.trim(),
    rules,
  };
}

export function joinRoomRequest(roomId: RoomId): ClientRequest {
  return { type: 'join_room', room_id: roomId.trim() };
}

export function spectateRoomRequest(roomId: RoomId): ClientRequest {
  return { type: 'spectate_room', room_id: roomId.trim() };
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

export function normalizeOptional(value: string): SessionToken | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}
