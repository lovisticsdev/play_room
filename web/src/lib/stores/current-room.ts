import { derived, writable } from 'svelte/store';
import type { Move, PlayerView, RoomSnapshot, RoundResult } from '../protocol/types';
import { isInRound, movesForGame } from '../protocol/rules';
import { sessionStore } from './session';

export interface CurrentRoomState {
  room: RoomSnapshot | null;
  localMove: Move | null;
  lastResult: RoundResult | null;
}

function sameRound(a: RoomSnapshot | null, b: RoomSnapshot): boolean {
  return a?.id === b.id && a.round === b.round;
}

function createCurrentRoomStore() {
  const { subscribe, set, update } = writable<CurrentRoomState>({ room: null, localMove: null, lastResult: null });

  return {
    subscribe,
    setRoom: (room: RoomSnapshot) => update((state) => {
      const keepRoundState = sameRound(state.room, room);
      const inRound = isInRound(room.phase);

      return {
        room,
        localMove: keepRoundState && inRound ? state.localMove : null,
        lastResult: keepRoundState ? state.lastResult : null,
      };
    }),
    setLocalMove: (move: Move | null) => update((state) => ({ ...state, localMove: move })),
    setRoundResult: (result: RoundResult) => update((state) => ({ ...state, lastResult: result, localMove: null })),
    clearRoundState: () => update((state) => ({ ...state, localMove: null, lastResult: null })),
    clear: () => set({ room: null, localMove: null, lastResult: null }),
  };
}

export const currentRoomStore = createCurrentRoomStore();

export const currentPlayer = derived(
  [currentRoomStore, sessionStore],
  ([$room, $session]): PlayerView | null => {
    if (!$room.room || !$session.playerId) return null;
    return $room.room.players.find((player) => player.id === $session.playerId) ?? null;
  },
);

export const allowedMoves = derived(currentRoomStore, ($room): Move[] => {
  if (!$room.room) return [];
  return movesForGame($room.room.rules.game);
});
