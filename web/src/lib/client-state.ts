import { writable } from 'svelte/store';
import type {
  RoomSnapshot,
  RoomSummary,
  ServerEvent,
  ServerMessage,
  ServerResult,
  WelcomeState,
} from './protocol';

export type LogLevel = 'info' | 'success' | 'warning' | 'error' | 'protocol';

export interface EventLogEntry {
  id: number;
  at: string;
  level: LogLevel;
  message: string;
  raw?: unknown;
}

export interface AppState {
  connected: boolean;
  connecting: boolean;
  serverUrl: string;
  displayName: string;
  welcome: WelcomeState | null;
  rooms: RoomSummary[];
  currentRoom: RoomSnapshot | null;
  eventLog: EventLogEntry[];
  lastError: string | null;
}

export const initialState: AppState = {
  connected: false,
  connecting: false,
  serverUrl: 'ws://127.0.0.1:7878/ws',
  displayName: 'alice',
  welcome: null,
  rooms: [],
  currentRoom: null,
  eventLog: [],
  lastError: null,
};

export const appState = writable<AppState>(initialState);

let nextLogId = 1;

export function pushLog(level: LogLevel, message: string, raw?: unknown): void {
  appState.update((state) => ({
    ...state,
    eventLog: [
      {
        id: nextLogId++,
        at: new Date().toLocaleTimeString(),
        level,
        message,
        raw,
      },
      ...state.eventLog,
    ].slice(0, 200),
  }));
}

export function applyServerResult(result: ServerResult): void {
  switch (result.status) {
    case 'welcome':
      appState.update((state) => ({
        ...state,
        welcome: {
          player_id: result.player_id,
          reconnect_token: result.reconnect_token,
          protocol_version: result.protocol_version,
        },
        lastError: null,
      }));
      pushLog('success', `Connected as ${result.player_id}`);
      break;
    case 'room_list':
      appState.update((state) => ({ ...state, rooms: result.rooms }));
      break;
    case 'room_snapshot':
      appState.update((state) => ({ ...state, currentRoom: result.room }));
      break;
    case 'error':
      appState.update((state) => ({ ...state, lastError: result.message }));
      pushLog('error', result.message, result);
      break;
    case 'pong':
      pushLog('success', 'Pong');
      break;
    case 'ok':
      pushLog('success', 'OK');
      break;
  }
}

export function applyServerEvent(event: ServerEvent): void {
  switch (event.type) {
    case 'notice':
      pushLog('info', event.message, event);
      break;
    case 'room_event':
      pushLog('protocol', `${event.room_id}: ${event.event.event}`, event);
      break;
    case 'room_snapshot':
      appState.update((state) => ({ ...state, currentRoom: event.room }));
      pushLog('protocol', `snapshot: ${event.room.name}`, event);
      break;
  }
}

export function applyServerMessage(message: ServerMessage): void {
  if (message.kind === 'response') {
    pushLog('protocol', `response:${message.request_id}:${message.result.status}`, message);
    applyServerResult(message.result);
  } else {
    applyServerEvent(message.event);
  }
}

export function setConnected(connected: boolean): void {
  appState.update((state) => ({ ...state, connected, connecting: false }));
}

export function setConnecting(connecting: boolean): void {
  appState.update((state) => ({ ...state, connecting }));
}

export function setConnectionError(message: string): void {
  appState.update((state) => ({ ...state, connecting: false, connected: false, lastError: message }));
  pushLog('error', message);
}

export function clearRuntimeState(): void {
  appState.update((state) => ({
    ...state,
    connected: false,
    connecting: false,
    welcome: null,
    rooms: [],
    currentRoom: null,
  }));
}
