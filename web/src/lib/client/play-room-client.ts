import { get } from 'svelte/store';
import { PlayRoomSocket } from './websocket';
import {
  connectRequest,
  createRoomRequest,
  enterRoomRequest,
  joinRoomRequest,
  leaveRoomRequest,
  listRoomsRequest,
  reconnectRequest,
  setReadyRequest,
  setSpectatorRequest,
  spectateRoomRequest,
  startNextMatchRequest,
  submitMoveRequest,
  updateDisplayNameRequest,
  updateMatchFormatRequest,
} from '../protocol/commands';
import type { ClientRequest, ErrorCode, GameRules, Move, RoomSnapshot, ServerEvent, ServerResult } from '../protocol/types';
import { connectionStore, DEFAULT_SERVER_URL } from '../stores/connection';
import { currentRoomStore } from '../stores/current-room';
import { eventLogStore } from '../stores/event-log';
import { roomsStore } from '../stores/rooms';
import { seatReservationsStore } from '../stores/seat-reservations';
import { sessionStore } from '../stores/session';
import { uiStore } from '../stores/ui';
import {
  clearReconnectToken,
  loadReconnectToken,
  loadServerUrl,
  saveDisplayName,
  saveReconnectToken,
  saveServerUrl,
} from '../storage/reconnect-token';
import { formatRoomEvent } from '../view/event-format';

type WelcomeResult = Extract<ServerResult, { status: 'welcome' }>;

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

const RECONNECT_DELAYS_MS = [0, 500, 1000, 2000, 4000, 8000] as const;
const RECONNECT_ROOM_SNAPSHOT_WAIT_MS = 750;

type ReconnectTrigger = 'startup' | 'transient';

type RoomSnapshotWaiter = {
  afterVersion: number;
  resolve: (restored: boolean) => void;
  timeoutId: ReturnType<typeof setTimeout>;
};

class PlayRoomClient {
  private socket = new PlayRoomSocket();
  private started = false;
  private manualClose = false;
  private reconnecting = false;
  private reconnectAttempt = 0;
  private reconnectTimerId: ReturnType<typeof setTimeout> | null = null;
  private cleanups: Array<() => void> = [];
  private roomPollId: ReturnType<typeof setInterval> | null = null;
  private pollingRooms = false;
  private roomSnapshotVersion = 0;
  private roomSnapshotWaiters: RoomSnapshotWaiter[] = [];

  start(): void {
    if (this.started) return;
    this.started = true;

    this.cleanups = [
      this.socket.onEvent((event) => this.applyServerEvent(event)),
      this.socket.onClose((event) => this.handleClose(event)),
      this.socket.onError((event) => this.handleSocketError(event)),
    ];

    void this.autoReconnect();
  }

  stop(): void {
    this.cleanups.forEach((cleanup) => cleanup());
    this.cleanups = [];
    this.cancelReconnect();
    this.manualClose = true;
    this.stopRoomPolling();
    this.socket.close();
    this.started = false;
  }

  async connect(serverUrl: string, displayName: string, reconnectToken: string | null = null): Promise<void> {
    this.cancelReconnect();
    const url = serverUrl.trim() || DEFAULT_SERVER_URL;
    const name = displayName.trim();
    const usingReconnect = Boolean(reconnectToken?.trim());

    if (!name && !usingReconnect) {
      throw new Error('Display name is required.');
    }

    connectionStore.setConnecting(url);

    await this.openSocket(url);
    const snapshotVersion = this.roomSnapshotVersion;
    const result = await this.request(connectRequest(name, reconnectToken));

    if (result.status !== 'welcome') {
      throw result.status === 'error' ? new PlayRoomRequestError(result) : new Error('Connect failed.');
    }

    this.applyServerResult(result);
    this.startRoomPolling();
    saveServerUrl(url);

    await this.refreshRooms();
    if (usingReconnect) {
      if (await this.restoreRoomAfterWelcome(result, snapshotVersion)) {
        uiStore.closeRoomsModal();
      } else {
        this.clearRoomPresence();
        uiStore.openRoomsModal('join');
        this.announceUnrestoredReconnect(result);
      }
    } else {
      uiStore.openRoomsModal('join');
    }
  }

  async autoReconnect(): Promise<void> {
    const token = loadReconnectToken();

    if (!token || !this.shouldAutoReconnect()) {
      uiStore.openRoomsModal('join');
      return;
    }

    this.beginStoredReconnect('startup');
  }

  disconnect(): void {
    this.cancelReconnect();
    this.manualClose = true;
    this.stopRoomPolling();
    this.socket.close();
    clearReconnectToken();
    sessionStore.clear();
    roomsStore.clear();
    this.clearRoomPresence();
    connectionStore.setDisconnected();
    uiStore.openRoomsModal('join');
    eventLogStore.push('warning', 'Disconnected');
  }

  async refreshRooms(options: { silent?: boolean } = {}): Promise<void> {
    if (!this.socket.isOpen) return;
    if (!options.silent) roomsStore.setLoading();

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

  async joinOrSpectateRoom(roomId: string): Promise<void> {
    await this.sendAndApply(enterRoomRequest(roomId, 'auto'));
    await this.refreshRooms();
    uiStore.closeRoomsModal();
  }

  async spectateRoom(roomId: string): Promise<void> {
    await this.sendAndApply(spectateRoomRequest(roomId));
    await this.refreshRooms();
    uiStore.closeRoomsModal();
  }

  async updateDisplayName(name: string): Promise<void> {
    const displayName = name.trim();
    if (!displayName) throw new Error('Display name is required.');

    await this.sendAndApply(updateDisplayNameRequest(displayName));
    sessionStore.updateDisplayName(displayName);
    saveDisplayName(displayName);
    eventLogStore.push('success', `Display name updated to ${displayName}`);
  }

  async updateMatchFormat(targetScore: number): Promise<void> {
    if (!Number.isSafeInteger(targetScore) || targetScore < 1) {
      throw new Error('Race target must be at least 1.');
    }

    await this.sendAndApply(updateMatchFormatRequest(targetScore));
  }

  async leaveRoom(): Promise<void> {
    await this.sendAndApply(leaveRoomRequest());
    this.clearRoomPresence();
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

  private async openSocket(
    serverUrl: string,
    options: { announce?: boolean; markConnected?: boolean } = {},
  ): Promise<void> {
    const announce = options.announce ?? true;
    const markConnected = options.markConnected ?? true;
    this.manualClose = false;
    await this.socket.connect(serverUrl);
    if (markConnected) connectionStore.setConnected(serverUrl);
    if (announce) eventLogStore.push('success', `WebSocket open: ${serverUrl}`);
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

  private beginStoredReconnect(trigger: ReconnectTrigger): void {
    if (this.reconnecting) return;

    const token = loadReconnectToken();
    const serverUrl = this.reconnectServerUrl();
    if (!token) {
      this.finishReconnectFailure(serverUrl, 'Connection lost. Reconnect token is missing.', true);
      return;
    }

    this.reconnecting = true;
    this.reconnectAttempt = 0;
    this.stopRoomPolling();
    connectionStore.setReconnecting(serverUrl);

    if (trigger === 'transient') {
      if (get(currentRoomStore).room) uiStore.closeRoomsModal();
      eventLogStore.push('warning', 'Connection lost; attempting reconnect');
    }

    this.scheduleReconnectAttempt(RECONNECT_DELAYS_MS[0]);
  }

  private scheduleReconnectAttempt(delayMs: number): void {
    this.clearReconnectTimer();
    this.reconnectTimerId = setTimeout(() => {
      this.reconnectTimerId = null;
      void this.runReconnectAttempt();
    }, delayMs);
  }

  private async runReconnectAttempt(): Promise<void> {
    if (!this.reconnecting) return;

    const token = loadReconnectToken();
    const serverUrl = this.reconnectServerUrl();
    if (!token) {
      this.finishReconnectFailure(serverUrl, 'Reconnect token is missing.', true);
      return;
    }

    const attempt = this.reconnectAttempt + 1;
    this.reconnectAttempt = attempt;
    const snapshotVersion = this.roomSnapshotVersion;

    try {
      await this.openSocket(serverUrl, { announce: false, markConnected: false });
      const result = await this.request(reconnectRequest(token));

      if (result.status !== 'welcome') {
        throw result.status === 'error' ? new PlayRoomRequestError(result) : new Error('Reconnect failed.');
      }

      this.applyServerResult(result);
      connectionStore.setConnected(serverUrl);
      this.reconnecting = false;
      this.reconnectAttempt = 0;
      this.startRoomPolling();
      await this.refreshRooms({ silent: true });

      if (await this.restoreRoomAfterWelcome(result, snapshotVersion)) {
        uiStore.closeRoomsModal();
        eventLogStore.push('success', 'Reconnected with stored token');
      } else {
        this.clearRoomPresence();
        uiStore.openRoomsModal('join');
        this.announceUnrestoredReconnect(result);
      }
    } catch (error) {
      if (!this.reconnecting) return;
      this.manualClose = true;
      this.socket.close();

      if (error instanceof PlayRoomRequestError) {
        this.finishReconnectFailure(serverUrl, error.message, true);
        return;
      }

      if (this.reconnectAttempt >= RECONNECT_DELAYS_MS.length) {
        const message = error instanceof Error ? error.message : 'Reconnect failed';
        this.finishReconnectFailure(serverUrl, message, false);
        return;
      }

      const nextDelay = RECONNECT_DELAYS_MS[this.reconnectAttempt];
      eventLogStore.push('warning', `Reconnect attempt ${attempt} failed; retrying`);
      this.scheduleReconnectAttempt(nextDelay);
    }
  }

  private cancelReconnect(): void {
    this.reconnecting = false;
    this.reconnectAttempt = 0;
    this.clearReconnectTimer();
  }

  private clearReconnectTimer(): void {
    if (!this.reconnectTimerId) return;
    clearTimeout(this.reconnectTimerId);
    this.reconnectTimerId = null;
  }

  private finishReconnectFailure(serverUrl: string, message: string, clearToken: boolean): void {
    this.cancelReconnect();
    this.stopRoomPolling();
    this.manualClose = true;
    this.socket.close();
    if (clearToken) {
      clearReconnectToken();
      sessionStore.clear();
    }
    roomsStore.clear();
    this.clearRoomPresence();
    connectionStore.setError(serverUrl, message);
    uiStore.openRoomsModal('join');
    eventLogStore.push('error', message);
  }

  private reconnectServerUrl(): string {
    return loadServerUrl(get(connectionStore).serverUrl || DEFAULT_SERVER_URL);
  }

  private async restoreRoomAfterWelcome(result: WelcomeResult, snapshotVersion: number): Promise<boolean> {
    if (!result.room_restored) return false;
    return this.waitForRoomSnapshotAfter(snapshotVersion);
  }

  private announceUnrestoredReconnect(result: WelcomeResult): void {
    if (result.stale_token_replaced) {
      eventLogStore.push('warning', 'Connected with a fresh session; previous room was not restored');
      return;
    }

    if (result.reconnected) {
      eventLogStore.push('warning', 'Reconnected identity, but no room was restored');
      return;
    }

    eventLogStore.push('info', 'Connected; no previous room was restored');
  }

  private waitForRoomSnapshotAfter(
    afterVersion: number,
    timeoutMs = RECONNECT_ROOM_SNAPSHOT_WAIT_MS,
  ): Promise<boolean> {
    if (this.roomSnapshotVersion > afterVersion && get(currentRoomStore).room) {
      return Promise.resolve(true);
    }

    return new Promise((resolve) => {
      const timeoutId = setTimeout(() => {
        this.roomSnapshotWaiters = this.roomSnapshotWaiters.filter((waiter) => waiter.resolve !== resolve);
        resolve(false);
      }, timeoutMs);

      this.roomSnapshotWaiters.push({ afterVersion, resolve, timeoutId });
    });
  }

  private noteRoomSnapshot(): void {
    this.roomSnapshotVersion += 1;
    const ready = this.roomSnapshotWaiters.filter((waiter) => this.roomSnapshotVersion > waiter.afterVersion);
    this.roomSnapshotWaiters = this.roomSnapshotWaiters.filter((waiter) => this.roomSnapshotVersion <= waiter.afterVersion);

    for (const waiter of ready) {
      clearTimeout(waiter.timeoutId);
      waiter.resolve(Boolean(get(currentRoomStore).room));
    }
  }

  private clearRoomPresence(): void {
    currentRoomStore.clear();
    seatReservationsStore.clear();
  }

  private startRoomPolling(): void {
    if (this.roomPollId) return;
    this.roomPollId = setInterval(() => {
      if (this.pollingRooms || !this.socket.isOpen) return;
      this.pollingRooms = true;
      void this.refreshRooms({ silent: true })
        .catch(() => {})
        .finally(() => {
          this.pollingRooms = false;
        });
    }, 2500);
  }

  private stopRoomPolling(): void {
    if (!this.roomPollId) return;
    clearInterval(this.roomPollId);
    this.roomPollId = null;
    this.pollingRooms = false;
  }

  private shouldAutoReconnect(): boolean {
    const [navigation] = performance.getEntriesByType('navigation') as PerformanceNavigationTiming[];
    return navigation?.type === 'reload';
  }

  private applyServerResult(result: ServerResult): void {
    switch (result.status) {
      case 'error':
        roomsStore.setError(result.message);
        eventLogStore.push('error', result.message, result);
        break;
      case 'welcome': {
        const displayName = result.display_name;
        sessionStore.setSession({
          playerId: result.player_id,
          displayName,
          reconnectToken: result.reconnect_token,
          protocolVersion: result.protocol_version,
        });
        saveReconnectToken(result.reconnect_token);
        saveDisplayName(displayName);
        eventLogStore.push('success', `Connected as ${displayName}`);
        break;
      }
      case 'room_list':
        roomsStore.setRooms(result.rooms);
        break;
      case 'room_snapshot':
        currentRoomStore.setRoom(result.room);
        roomsStore.upsertFromSnapshot(result.room);
        seatReservationsStore.syncRoom(result.room);
        this.syncLocalDisplayName(result.room);
        this.noteRoomSnapshot();
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
        roomsStore.upsertFromSnapshot(event.room);
        seatReservationsStore.syncRoom(event.room);
        this.syncLocalDisplayName(event.room);
        this.noteRoomSnapshot();
        break;
      case 'room_event': {
        const state = get(currentRoomStore);
        eventLogStore.push('protocol', formatRoomEvent(event.event, state.room), event);

        if (event.event.event === 'player_disconnected') {
          seatReservationsStore.markDisconnected(event.event.player_id);
        }

        if (
          event.event.event === 'player_left'
          || event.event.event === 'player_reconnected'
          || event.event.event === 'player_renamed'
          || (event.event.event === 'role_changed' && event.event.role === 'spectator')
        ) {
          seatReservationsStore.clearPlayer(event.event.player_id);
        }

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

  private syncLocalDisplayName(room: RoomSnapshot): void {
    const state = get(sessionStore);
    if (!state.playerId) return;

    const local = room.players.find((player) => player.id === state.playerId);
    if (!local || state.displayName === local.name) return;

    sessionStore.updateDisplayName(local.name);
    saveDisplayName(local.name);
  }

  private handleSocketError(event: Event): void {
    const message = event instanceof ErrorEvent && event.message ? event.message : 'WebSocket error';
    if (this.reconnecting) {
      eventLogStore.push('warning', message);
      return;
    }

    const connection = get(connectionStore);
    connectionStore.setError(connection.serverUrl, message);
    eventLogStore.push('error', message);
  }

  private handleClose(event: CloseEvent): void {
    if (this.manualClose) {
      this.manualClose = false;
      return;
    }

    this.stopRoomPolling();
    if (this.reconnecting) return;

    if (event.code === 1002) {
      this.finishReconnectFailure(this.reconnectServerUrl(), 'Protocol error: invalid server message.', false);
      return;
    }

    this.beginStoredReconnect('transient');
  }
}

export const playRoomClient = new PlayRoomClient();
