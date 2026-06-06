import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { listRoomsRequest } from '../protocol/commands';
import { PlayRoomSocket } from './websocket';

type ListenerEntry = {
  listener: (event: unknown) => void;
  once: boolean;
};

class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  static instances: FakeWebSocket[] = [];

  readonly url: string;
  readyState = FakeWebSocket.CONNECTING;
  throwOnSend = false;
  sent: string[] = [];
  private listeners = new Map<string, ListenerEntry[]>();

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  addEventListener(
    type: string,
    listener: (event: unknown) => void,
    options: { once?: boolean } = {},
  ): void {
    const entries = this.listeners.get(type) ?? [];
    entries.push({ listener, once: Boolean(options.once) });
    this.listeners.set(type, entries);
  }

  send(data: string): void {
    if (this.throwOnSend) throw new Error('send failed');
    this.sent.push(data);
  }

  close(): void {
    if (this.readyState === FakeWebSocket.CLOSED) return;
    this.readyState = FakeWebSocket.CLOSED;
    this.emit('close', { code: 1000, reason: '', wasClean: true });
  }

  open(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.emit('open', {});
  }

  emit(type: string, event: unknown): void {
    const entries = [...(this.listeners.get(type) ?? [])];
    for (const entry of entries) {
      entry.listener(event);
    }
    this.listeners.set(
      type,
      (this.listeners.get(type) ?? []).filter((entry) => !entry.once),
    );
  }
}

const originalWebSocket = globalThis.WebSocket;

function latestSocket(): FakeWebSocket {
  const socket = FakeWebSocket.instances.at(-1);
  if (!socket) throw new Error('expected fake websocket instance');
  return socket;
}

function pendingSize(socket: PlayRoomSocket): number {
  return (socket as unknown as { pending: Map<number, unknown> }).pending.size;
}

describe('PlayRoomSocket', () => {
  beforeEach(() => {
    FakeWebSocket.instances = [];
    Object.defineProperty(globalThis, 'WebSocket', {
      configurable: true,
      writable: true,
      value: FakeWebSocket,
    });
  });

  afterEach(() => {
    Object.defineProperty(globalThis, 'WebSocket', {
      configurable: true,
      writable: true,
      value: originalWebSocket,
    });
  });

  it('rejects connect when the socket closes before opening', async () => {
    const client = new PlayRoomSocket();
    const connected = client.connect('ws://play-room.test');

    latestSocket().close();

    await expect(connected).rejects.toThrow(/closed before opening/);
  });

  it('resolves connect only once if the socket opens and later closes', async () => {
    const client = new PlayRoomSocket();
    const connected = client.connect('ws://play-room.test');
    const socket = latestSocket();

    socket.open();
    await expect(connected).resolves.toBeUndefined();

    socket.close();
    await expect(connected).resolves.toBeUndefined();
  });

  it('rejects and clears pending requests when send throws', async () => {
    const client = new PlayRoomSocket();
    const connected = client.connect('ws://play-room.test');
    const socket = latestSocket();
    socket.open();
    await connected;
    socket.throwOnSend = true;

    const request = client.request(listRoomsRequest());

    await expect(request).rejects.toThrow(/send failed/);
    expect(pendingSize(client)).toBe(0);
  });

  it('rejects all pending requests on close', async () => {
    const client = new PlayRoomSocket();
    const connected = client.connect('ws://play-room.test');
    latestSocket().open();
    await connected;

    const request = client.request(listRoomsRequest());
    expect(pendingSize(client)).toBe(1);

    client.close(new Error('closed by test'));

    await expect(request).rejects.toThrow(/closed by test/);
    expect(pendingSize(client)).toBe(0);
  });
});
