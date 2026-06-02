import { writable } from 'svelte/store';
import type { RoomSnapshot, RoomSummary } from '../protocol/types';

export interface RoomsState {
  rooms: RoomSummary[];
  loading: boolean;
  error: string | null;
}

function summaryFromSnapshot(room: RoomSnapshot): RoomSummary {
  return {
    id: room.id,
    name: room.name,
    phase: room.phase,
    players: room.players.filter((player) => player.role === 'participant').length,
    spectators: room.players.filter((player) => player.role === 'spectator').length,
    max_players: room.rules.max_players,
    game: room.rules.game,
    target_score: room.rules.target_score,
  };
}

function sortRooms(rooms: RoomSummary[]): RoomSummary[] {
  return [...rooms].sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
}

function createRoomsStore() {
  const { subscribe, set, update } = writable<RoomsState>({ rooms: [], loading: false, error: null });

  return {
    subscribe,
    setLoading: () => update((state) => ({ ...state, loading: true, error: null })),
    setRooms: (rooms: RoomSummary[]) => set({ rooms: sortRooms(rooms), loading: false, error: null }),
    upsertFromSnapshot: (room: RoomSnapshot) => update((state) => {
      const summary = summaryFromSnapshot(room);
      const rooms = state.rooms.some((existing) => existing.id === summary.id)
        ? state.rooms.map((existing) => existing.id === summary.id ? summary : existing)
        : [...state.rooms, summary];
      return { rooms: sortRooms(rooms), loading: false, error: null };
    }),
    setError: (error: string) => update((state) => ({ ...state, loading: false, error })),
    clear: () => set({ rooms: [], loading: false, error: null }),
  };
}

export const roomsStore = createRoomsStore();
