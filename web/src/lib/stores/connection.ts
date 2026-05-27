import { writable } from 'svelte/store';
import type { WelcomeState } from '../protocol/types';

interface ConnectionState {
  connected: boolean;
  connecting: boolean;
  welcome: WelcomeState | null;
  error: string | null;
}

export const connectionStore = writable<ConnectionState>({
  connected: false,
  connecting: false,
  welcome: null,
  error: null,
});

export const connectionActions = {
  setConnecting: () => connectionStore.update(s => ({ ...s, connecting: true, error: null })),
  setConnected: (welcome: WelcomeState) => connectionStore.update(s => ({ ...s, connected: true, connecting: false, welcome, error: null })),
  setError: (error: string) => connectionStore.update(s => ({ ...s, connected: false, connecting: false, error })),
  disconnect: () => connectionStore.update(s => ({ ...s, connected: false, connecting: false })),
};