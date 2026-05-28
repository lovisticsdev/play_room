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
    this.close();

    return new Promise((resolve, reject) => {
      const socket = new WebSocket(url);
      this.socket = socket;

      socket.addEventListener('open', () => resolve(), { once: true });
      socket.addEventListener(
        'error',
        (event) => {
          this.errorListeners.forEach((listener) => listener(event));
          reject(new Error(`WebSocket connection failed: ${url}`));
        },
        { once: true },
      );

      socket.addEventListener('message', (event) => this.handleMessage(event.data));
      socket.addEventListener('close', (event) => {
        this.pending.forEach(({ reject: rejectPending, timeoutId }) => {
          clearTimeout(timeoutId);
          rejectPending(new Error('WebSocket closed'));
        });
        this.pending.clear();
        this.closeListeners.forEach((listener) => listener(event));
      });
    });
  }

  request(request: ClientRequest): Promise<ServerResult> {
    if (!this.isOpen) return Promise.reject(new Error('Socket is not open'));

    const request_id = this.nextRequestId;
    this.nextRequestId += 1;

    const envelope: ClientEnvelope = { request_id, request };

    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pending.delete(request_id);
        reject(new Error(`Request timeout: ${request_id}`));
      }, REQUEST_TIMEOUT_MS);

      this.pending.set(request_id, { resolve, reject, timeoutId });
      this.socket?.send(JSON.stringify(envelope));
    });
  }

  close(): void {
    if (this.socket && this.socket.readyState !== WebSocket.CLOSED) {
      this.socket.close();
    }
    this.socket = null;
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

  private handleMessage(raw: unknown): void {
    let message: ServerMessage;

    try {
      message = JSON.parse(String(raw)) as ServerMessage;
    } catch {
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
}
