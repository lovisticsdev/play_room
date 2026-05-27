import { writable } from 'svelte/store';
import type { RoomSummary } from '../protocol/types';

interface LobbyState {
  rooms: RoomSummary[];
}

export const lobbyStore = writable<LobbyState>({ rooms: [] });

export const lobbyActions = {
  setRooms: (rooms: RoomSummary[]) => lobbyStore.set({ rooms }),
  clear: () => lobbyStore.set({ rooms: [] }),
};