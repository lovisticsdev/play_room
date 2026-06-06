import { writable } from 'svelte/store';
import type { PlayerId, SessionToken } from '../protocol/types';

export interface SessionState {
  playerId: PlayerId | null;
  displayName: string | null;
  reconnectToken: SessionToken | null;
  protocolVersion: number | null;
}

function createSessionStore() {
  const { subscribe, set, update } = writable<SessionState>({
    playerId: null,
    displayName: null,
    reconnectToken: null,
    protocolVersion: null,
  });

  return {
    subscribe,
    setSession: (state: SessionState) => set(state),
    setDisplayName: (displayName: string | null) => update((state) => ({ ...state, displayName })),
    updateDisplayName: (displayName: string) => update((state) => ({ ...state, displayName })),
    clear: () => set({ playerId: null, displayName: null, reconnectToken: null, protocolVersion: null }),
  };
}

export const sessionStore = createSessionStore();
