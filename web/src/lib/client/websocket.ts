import { decodeServerMessage, ProtocolDecodeError } from '../protocol/decode';
import type { ClientEnvelope, ClientRequest, ServerEvent, ServerMessage, ServerResult } from '../protocol/types';

type PendingRequest = {
  resolve: (result: ServerResult) => void;
  reject: (error: Error) => void;
  timeoutId: ReturnType<typeof setTimeout>;
};

type Listener<T> = (value: T) => void;

const REQUEST_TIMEOUT_MS = 10000;

export class PlayRoomSocket {
  private socket: WebSocket | null = null;
  private connectionGeneration = 0;
  private nextRequestId = 1;
  private pending = new Map<number, PendingRequest>();
  private messageListeners = new Set<Listener<ServerMessage>>();
  private eventListeners = new Set<Listener<ServerEvent>>();
  private closeListeners = new Set<Listener<CloseEvent>>();
  private errorListeners = new Set<Listener<Event>>();

  get isOpen(): boolean {
    return this.socket?.readyState === WebSocket.OPEN;
  }

  connect(url: string): Promise<void> {
    this.close(new Error('Socket replaced by a new connection'));
    const generation = this.connectionGeneration + 1;
    this.connectionGeneration = generation;

    return new Promise((resolve, reject) => {
      let settled = false;
      const resolveOnce = () => {
        if (settled) return;
        settled = true;
        resolve();
      };
      const rejectOnce = (error: Error) => {
        if (settled) return;
        settled = true;
        reject(error);
      };
      const socket = new WebSocket(url);
      this.socket = socket;

      socket.addEventListener(
        'open',
        () => {
          if (!this.isCurrentSocket(socket, generation)) return;
          resolveOnce();
        },
        { once: true },
      );

      socket.addEventListener(
        'error',
        (event) => {
          if (!this.isCurrentSocket(socket, generation)) return;
          this.errorListeners.forEach((listener) => listener(event));
          rejectOnce(new Error(`WebSocket connection failed: ${url}`));
        },
        { once: true },
      );

      socket.addEventListener('message', (event) => {
        if (!this.isCurrentSocket(socket, generation)) return;
        this.handleMessage(event.data);
      });

      socket.addEventListener('close', (event) => {
        if (!this.isCurrentOrClosedSocket(socket, generation)) return;
        if (this.socket === socket) this.socket = null;
        this.rejectPending(new Error('WebSocket closed'));
        this.closeListeners.forEach((listener) => listener(event));
        rejectOnce(new Error('WebSocket closed before opening'));
      });
    });
  }

  request(request: ClientRequest): Promise<ServerResult> {
    if (!this.isOpen || !this.socket) return Promise.reject(new Error('Socket is not open'));

    const request_id = this.nextRequestId;
    this.nextRequestId += 1;

    const envelope: ClientEnvelope = { request_id, request };
    const socket = this.socket;

    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pending.delete(request_id);
        reject(new Error(`Request timeout: ${request_id}`));
      }, REQUEST_TIMEOUT_MS);

      this.pending.set(request_id, { resolve, reject, timeoutId });
      try {
        socket.send(JSON.stringify(envelope));
      } catch (error) {
        clearTimeout(timeoutId);
        this.pending.delete(request_id);
        reject(error instanceof Error ? error : new Error('WebSocket send failed'));
      }
    });
  }

  close(error = new Error('WebSocket closed')): void {
    const socket = this.socket;
    if (!socket) return;

    this.socket = null;
    this.rejectPending(error);

    if (socket.readyState !== WebSocket.CLOSING && socket.readyState !== WebSocket.CLOSED) {
      socket.close();
    }
  }

  onMessage(listener: Listener<ServerMessage>): () => void {
    this.messageListeners.add(listener);
    return () => this.messageListeners.delete(listener);
  }

  onEvent(listener: Listener<ServerEvent>): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  onClose(listener: Listener<CloseEvent>): () => void {
    this.closeListeners.add(listener);
    return () => this.closeListeners.delete(listener);
  }

  onError(listener: Listener<Event>): () => void {
    this.errorListeners.add(listener);
    return () => this.errorListeners.delete(listener);
  }

  private isCurrentSocket(socket: WebSocket, generation: number): boolean {
    return this.connectionGeneration === generation && this.socket === socket;
  }

  private isCurrentOrClosedSocket(socket: WebSocket, generation: number): boolean {
    return this.connectionGeneration === generation && (this.socket === socket || this.socket === null);
  }

  private handleMessage(raw: unknown): void {
    let message: ServerMessage;

    try {
      message = decodeServerMessage(raw);
    } catch (error) {
      const decodeError = error instanceof ProtocolDecodeError
        ? error
        : new ProtocolDecodeError('Server message could not be decoded.');
      this.rejectPending(decodeError);
      this.emitError(decodeError);
      this.socket?.close(1002, 'invalid server message');
      return;
    }

    this.messageListeners.forEach((listener) => listener(message));

    if (message.kind === 'response') {
      const pending = this.pending.get(message.request_id);
      if (!pending) return;

      clearTimeout(pending.timeoutId);
      this.pending.delete(message.request_id);
      pending.resolve(message.result);
      return;
    }

    this.eventListeners.forEach((listener) => listener(message.event));
  }

  private rejectPending(error: Error): void {
    this.pending.forEach(({ reject, timeoutId }) => {
      clearTimeout(timeoutId);
      reject(error);
    });
    this.pending.clear();
  }

  private emitError(error: Error): void {
    const event = typeof ErrorEvent === 'function'
      ? new ErrorEvent('error', { error, message: error.message })
      : new Event('error');
    this.errorListeners.forEach((listener) => listener(event));
  }
}
