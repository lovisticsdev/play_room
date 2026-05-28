import { get } from 'svelte/store';
import { PlayRoomSocket } from './websocket';
import {
  connectRequest,
  createRoomRequest,
  joinRoomRequest,
  leaveRoomRequest,
  listRoomsRequest,
  reconnectRequest,
  setReadyRequest,
  setSpectatorRequest,
  spectateRoomRequest,
  startNextMatchRequest,
  submitMoveRequest,
} from '../protocol/commands';
import type { ClientRequest, ErrorCode, GameRules, Move, ServerEvent, ServerResult } from '../protocol/types';
import { connectionStore, DEFAULT_SERVER_URL } from '../stores/connection';
import { currentRoomStore } from '../stores/current-room';
import { eventLogStore } from '../stores/event-log';
import { roomsStore } from '../stores/rooms';
import { sessionStore } from '../stores/session';
import { uiStore } from '../stores/ui';
import {
  clearReconnectToken,
  loadDisplayName,
  loadReconnectToken,
  loadServerUrl,
  saveDisplayName,
  saveReconnectToken,
  saveServerUrl,
} from '../storage/reconnect-token';
import { formatRoomEvent } from '../view/event-format';

export class PlayRoomRequestError extends Error {
  readonly code: ErrorCode | null;
  readonly suggestions: string[];

  constructor(result: Extract<ServerResult, { status: 'error' }>) {
    super(result.message);
    this.name = 'PlayRoomRequestError';
    this.code = result.code ?? null;
    this.suggestions = result.suggestions ?? [];
  }
}

export function isPlayRoomRequestError(error: unknown): error is PlayRoomRequestError {
  return error instanceof PlayRoomRequestError;
}

class PlayRoomClient {
  private socket = new PlayRoomSocket();
  private started = false;
  private manualClose = false;
  private cleanups: Array<() => void> = [];
  private pendingDisplayName: string | null = null;

  start(): void {
    if (this.started) return;
    this.started = true;

    this.cleanups = [
      this.socket.onEvent((event) => this.applyServerEvent(event)),
      this.socket.onClose(() => this.handleClose()),
      this.socket.onError(() => {
        const connection = get(connectionStore);
        connectionStore.setError(connection.serverUrl, 'WebSocket error');
      }),
    ];

    void this.autoReconnect();
  }

  stop(): void {
    this.cleanups.forEach((cleanup) => cleanup());
    this.cleanups = [];
    this.manualClose = true;
    this.socket.close();
    this.started = false;
  }

  async connect(serverUrl: string, displayName: string, reconnectToken: string | null = null): Promise<void> {
    const url = serverUrl.trim() || DEFAULT_SERVER_URL;
    const name = displayName.trim();
    const usingReconnect = Boolean(reconnectToken?.trim());

    if (!name && !usingReconnect) {
      throw new Error('Display name is required.');
    }

    this.pendingDisplayName = name || loadDisplayName() || 'player';
    connectionStore.setConnecting(url);

    await this.openSocket(url);
    const result = await this.request(connectRequest(name, reconnectToken));

    if (result.status !== 'welcome') {
      throw result.status === 'error' ? new PlayRoomRequestError(result) : new Error('Connect failed.');
    }

    this.applyServerResult(result);
    saveServerUrl(url);
    if (name) saveDisplayName(name);

    await this.refreshRooms();
    if (usingReconnect && (await this.waitForCurrentRoom())) {
      uiStore.closeRoomsModal();
    } else {
      uiStore.openRoomsModal('join');
    }
  }

  async autoReconnect(): Promise<void> {
    const token = loadReconnectToken();
    const serverUrl = loadServerUrl(DEFAULT_SERVER_URL);

    if (!token) {
      uiStore.openRoomsModal('join');
      return;
    }

    this.pendingDisplayName = loadDisplayName() || 'reconnected player';
    connectionStore.setReconnecting(serverUrl);

    try {
      await this.openSocket(serverUrl);
      const result = await this.request(reconnectRequest(token));

      if (result.status !== 'welcome') {
        throw result.status === 'error' ? new PlayRoomRequestError(result) : new Error('Reconnect failed.');
      }

      this.applyServerResult(result);
      await this.refreshRooms();
      if (await this.waitForCurrentRoom()) {
        uiStore.closeRoomsModal();
      } else {
        uiStore.openRoomsModal('join');
      }
      eventLogStore.push('success', 'Reconnected with stored token');
    } catch (error) {
      clearReconnectToken();
      sessionStore.clear();
      currentRoomStore.clear();
      connectionStore.setError(serverUrl, error instanceof Error ? error.message : 'Reconnect failed');
      uiStore.openRoomsModal('join');
    }
  }

  disconnect(): void {
    this.manualClose = true;
    this.socket.close();
    clearReconnectToken();
    sessionStore.clear();
    roomsStore.clear();
    currentRoomStore.clear();
    connectionStore.setDisconnected();
    uiStore.openRoomsModal('join');
    eventLogStore.push('warning', 'Disconnected');
  }

  async refreshRooms(): Promise<void> {
    roomsStore.setLoading();
    const result = await this.request(listRoomsRequest());
    this.applyServerResult(result);
    if (result.status === 'error') throw new PlayRoomRequestError(result);
  }

  async createRoom(name: string, rules: GameRules | null = null): Promise<void> {
    await this.sendAndApply(createRoomRequest(name, rules));
    await this.refreshRooms();
    uiStore.closeRoomsModal();
  }

  async joinRoom(roomId: string): Promise<void> {
    await this.sendAndApply(joinRoomRequest(roomId));
    await this.refreshRooms();
    uiStore.closeRoomsModal();
  }

  async spectateRoom(roomId: string): Promise<void> {
    await this.sendAndApply(spectateRoomRequest(roomId));
    await this.refreshRooms();
    uiStore.closeRoomsModal();
  }

  async leaveRoom(): Promise<void> {
    await this.sendAndApply(leaveRoomRequest());
    currentRoomStore.clear();
    await this.refreshRooms();
    uiStore.openRoomsModal('join');
  }

  async startNextMatch(): Promise<void> {
    await this.sendAndApply(startNextMatchRequest());
  }

  async setReady(ready: boolean): Promise<void> {
    await this.sendAndApply(setReadyRequest(ready));
  }

  async setSpectator(spectator: boolean): Promise<void> {
    await this.sendAndApply(setSpectatorRequest(spectator));
  }

  async submitMove(move: Move): Promise<void> {
    const result = await this.sendAndApply(submitMoveRequest(move));
    if (result.status === 'ok') currentRoomStore.setLocalMove(move);
  }

  private async openSocket(serverUrl: string): Promise<void> {
    this.manualClose = false;
    await this.socket.connect(serverUrl);
    connectionStore.setConnected(serverUrl);
    eventLogStore.push('success', `WebSocket open: ${serverUrl}`);
  }

  private async sendAndApply(request: ClientRequest): Promise<ServerResult> {
    const result = await this.request(request);
    this.applyServerResult(result);
    if (result.status === 'error') throw new PlayRoomRequestError(result);
    return result;
  }

  private async request(request: ClientRequest): Promise<ServerResult> {
    try {
      const startedAt = performance.now();
      const result = await this.socket.request(request);
      connectionStore.setLatency(Math.max(0, Math.round(performance.now() - startedAt)));
      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Request failed';
      eventLogStore.push('error', message);
      throw error;
    }
  }

  private waitForCurrentRoom(timeoutMs = 500): Promise<boolean> {
    if (get(currentRoomStore).room) return Promise.resolve(true);

    return new Promise((resolve) => {
      let settled = false;
      let unsubscribe: (() => void) | null = null;
      const timeoutId = setTimeout(() => finish(false), timeoutMs);

      const finish = (restored: boolean) => {
        if (settled) return;
        settled = true;
        if (unsubscribe) unsubscribe();
        clearTimeout(timeoutId);
        resolve(restored);
      };

      unsubscribe = currentRoomStore.subscribe((state) => {
        if (state.room) finish(true);
      });
    });
  }

  private applyServerResult(result: ServerResult): void {
    switch (result.status) {
      case 'error':
        roomsStore.setError(result.message);
        eventLogStore.push('error', result.message, result);
        break;
      case 'welcome': {
        const storedDisplayName = loadDisplayName();
        const displayName = this.pendingDisplayName ?? (storedDisplayName || null);
        sessionStore.setSession({
          playerId: result.player_id,
          displayName,
          reconnectToken: result.reconnect_token,
          protocolVersion: result.protocol_version,
        });
        saveReconnectToken(result.reconnect_token);
        eventLogStore.push('success', `Connected as ${displayName ?? result.player_id}`);
        break;
      }
      case 'room_list':
        roomsStore.setRooms(result.rooms);
        break;
      case 'room_snapshot':
        currentRoomStore.setRoom(result.room);
        eventLogStore.push('protocol', `Snapshot: ${result.room.name}`, result);
        break;
      case 'pong':
        eventLogStore.push('success', 'Pong');
        break;
      case 'ok':
        break;
    }
  }

  private applyServerEvent(event: ServerEvent): void {
    switch (event.type) {
      case 'notice':
        eventLogStore.push('info', event.message, event);
        break;
      case 'room_snapshot':
        currentRoomStore.setRoom(event.room);
        break;
      case 'room_event': {
        const state = get(currentRoomStore);
        eventLogStore.push('protocol', formatRoomEvent(event.event, state.room), event);

        if (event.event.event === 'round_started' || event.event.event === 'match_reset') {
          currentRoomStore.clearRoundState();
        }

        if (event.event.event === 'round_resolved') {
          currentRoomStore.setRoundResult(event.event.result);
        }
        break;
      }
    }
  }

  private handleClose(): void {
    if (this.manualClose) {
      this.manualClose = false;
      return;
    }

    connectionStore.setDisconnected();
    currentRoomStore.clear();
    roomsStore.clear();
    eventLogStore.push('warning', 'WebSocket closed');
    uiStore.openRoomsModal('join');
  }
}

export const playRoomClient = new PlayRoomClient();