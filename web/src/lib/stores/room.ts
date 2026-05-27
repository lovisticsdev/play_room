import { writable, derived } from 'svelte/store';
import type { Move, RoomSnapshot } from '../protocol/types';
import { connectionStore } from './connection';
import { isInRound } from '../protocol/rules';

interface RoomState {
  currentRoom: RoomSnapshot | null;
}

export const roomStore = writable<RoomState>({ currentRoom: null });

export const roomActions = {
  setRoom: (room: RoomSnapshot) => roomStore.set({ currentRoom: room }),
  clear: () => roomStore.set({ currentRoom: null }),
};

// Derived projections (Pure View State)
export const currentPlayer = derived(
  [roomStore, connectionStore],
  ([$room, $conn]) => $conn.welcome && $room.currentRoom
    ? $room.currentRoom.players.find(p => p.id === $conn.welcome!.player_id) ?? null
    : null
);

export const isParticipant = derived(currentPlayer, $p => $p?.role === 'participant');
export const isReady = derived(currentPlayer, $p => $p?.ready ?? false);
export const activeRound = derived(roomStore, $r => $r.currentRoom ? isInRound($r.currentRoom.phase) : false);

export const allowedMoves = derived(roomStore, $r => {
  if (!$r.currentRoom) return [];
  const baseMoves: Move[] = ['rock', 'paper', 'scissors', 'lizard', 'spock'];
  return $r.currentRoom.rules.game === 'rock_paper_scissors' ? baseMoves.slice(0, 3) : baseMoves;
});
