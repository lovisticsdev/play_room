import { writable } from 'svelte/store';
import type { RoomSummary } from '../protocol/types';

export interface RoomsState {
  rooms: RoomSummary[];
  loading: boolean;
  error: string | null;
}

function createRoomsStore() {
  const { subscribe, set, update } = writable<RoomsState>({ rooms: [], loading: false, error: null });

  return {
    subscribe,
    setLoading: () => update((state) => ({ ...state, loading: true, error: null })),
    setRooms: (rooms: RoomSummary[]) => set({ rooms, loading: false, error: null }),
    setError: (error: string) => update((state) => ({ ...state, loading: false, error })),
    clear: () => set({ rooms: [], loading: false, error: null }),
  };
}

export const roomsStore = createRoomsStore();
