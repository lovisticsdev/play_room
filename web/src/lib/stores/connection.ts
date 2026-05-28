import { writable } from 'svelte/store';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';

export interface ConnectionState {
  status: ConnectionStatus;
  serverUrl: string;
  latencyMs: number | null;
  error: string | null;
}

const DEFAULT_SERVER_URL = 'ws://127.0.0.1:7878/ws';

function createConnectionStore() {
  const { subscribe, set, update } = writable<ConnectionState>({
    status: 'disconnected',
    serverUrl: DEFAULT_SERVER_URL,
    latencyMs: null,
    error: null,
  });

  return {
    subscribe,
    setConnecting: (serverUrl: string) => set({ status: 'connecting', serverUrl, latencyMs: null, error: null }),
    setReconnecting: (serverUrl: string) => set({ status: 'reconnecting', serverUrl, latencyMs: null, error: null }),
    setConnected: (serverUrl: string) => update((state) => ({ ...state, status: 'connected', serverUrl, error: null })),
    setLatency: (latencyMs: number) => update((state) => ({ ...state, latencyMs })),
    setError: (serverUrl: string, error: string) => set({ status: 'error', serverUrl, latencyMs: null, error }),
    setDisconnected: () => update((state) => ({ ...state, status: 'disconnected', latencyMs: null, error: null })),
    reset: () => set({ status: 'disconnected', serverUrl: DEFAULT_SERVER_URL, latencyMs: null, error: null }),
  };
}

export const connectionStore = createConnectionStore();
export { DEFAULT_SERVER_URL };
