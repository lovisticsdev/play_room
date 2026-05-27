import { socket } from './websocket';
import { connectionActions } from '../stores/connection';
import { roomActions } from '../stores/room';
import { lobbyActions } from '../stores/lobby';
import { logStore } from '../stores/event-log';
import { leaveRoomRequest, listRoomsRequest } from '../protocol/commands';
import type { ClientRequest, ServerEvent, ServerResult } from '../protocol/types';

export function applyServerResult(result: ServerResult): void {
  switch (result.status) {
    case 'error':
      logStore.push('error', result.message);
      break;
    case 'welcome':
      connectionActions.setConnected(result);
      logStore.push('success', `Welcome! Player ID: ${result.player_id}`);
      break;
    case 'room_list':
      lobbyActions.setRooms(result.rooms);
      break;
    case 'room_snapshot':
      roomActions.setRoom(result.room);
      logStore.push('protocol', `snapshot: ${result.room.name}`, result);
      break;
    case 'pong':
      logStore.push('success', 'Pong');
      break;
    case 'ok':
      logStore.push('success', 'OK');
      break;
    default: {
      const exhaustive: never = result;
      return exhaustive;
    }
  }
}

export function applyServerEvent(event: ServerEvent): void {
  switch (event.type) {
    case 'notice':
      logStore.push('info', event.message, event);
      break;
    case 'room_event':
      logStore.push('protocol', `${event.room_id}: ${event.event.event}`, event);
      break;
    case 'room_snapshot':
      roomActions.setRoom(event.room);
      logStore.push('protocol', `snapshot: ${event.room.name}`, event);
      break;
    default: {
      const exhaustive: never = event;
      return exhaustive;
    }
  }
}

export function initDispatcher(): () => void {
  const unsubEvent = socket.onEvent(applyServerEvent);
  const unsubClose = socket.onClose((event) => {
    connectionActions.disconnect();
    roomActions.clear();
    lobbyActions.clear();
    logStore.push('warning', `WebSocket closed (${event.code})`);
  });
  const unsubError = socket.onError(() => {
    connectionActions.setError('WebSocket error');
  });

  return () => {
    unsubEvent();
    unsubClose();
    unsubError();
  };
}

type DispatchType =
  | 'SYSTEM_CONNECT'
  | 'SYSTEM_DISCONNECT'
  | 'SYSTEM_REFRESH_ROOMS'
  | 'SYSTEM_LEAVE_ROOM'
  | 'NETWORK_REQUEST';

type ConnectPayload = { url: string; request: ClientRequest };
type DispatchPayload = ConnectPayload | ClientRequest;

function requireConnectPayload(payload: DispatchPayload | undefined): ConnectPayload {
  if (!payload || !('url' in payload)) {
    throw new Error('Missing connect payload');
  }
  return payload;
}

function requireRequestPayload(payload: DispatchPayload | undefined): ClientRequest {
  if (!payload || 'url' in payload) {
    throw new Error('Missing request payload');
  }
  return payload;
}

export async function dispatch(type: DispatchType, payload?: DispatchPayload): Promise<void> {
  try {
    switch (type) {
      case 'SYSTEM_CONNECT': {
        const { url, request } = requireConnectPayload(payload);
        connectionActions.setConnecting();
        await socket.connect(url);
        logStore.push('success', `WebSocket open: ${url}`);

        applyServerResult(await socket.request(request));
        applyServerResult(await socket.request(listRoomsRequest()));
        break;
      }
      case 'SYSTEM_DISCONNECT': {
        socket.close();
        connectionActions.disconnect();
        roomActions.clear();
        lobbyActions.clear();
        logStore.push('warning', 'Disconnected manually');
        break;
      }
      case 'SYSTEM_REFRESH_ROOMS': {
        applyServerResult(await socket.request(listRoomsRequest()));
        break;
      }
      case 'SYSTEM_LEAVE_ROOM': {
        const result = await socket.request(leaveRoomRequest());
        applyServerResult(result);
        if (result.status === 'ok') {
          roomActions.clear();
          await dispatch('SYSTEM_REFRESH_ROOMS');
        }
        break;
      }
      case 'NETWORK_REQUEST': {
        const request = requireRequestPayload(payload);
        const result = await socket.request(request);
        applyServerResult(result);

        if (result.status === 'ok' && ['create_room', 'join_room', 'spectate_room'].includes(request.type)) {
          await dispatch('SYSTEM_REFRESH_ROOMS');
        }
        break;
      }
      default: {
        const exhaustive: never = type;
        return exhaustive;
      }
    }
  } catch (error) {
    if (type === 'SYSTEM_CONNECT') {
      connectionActions.setError(error instanceof Error ? error.message : 'Connection failed');
    } else {
      logStore.push('error', error instanceof Error ? error.message : 'Request failed');
    }
  }
}
