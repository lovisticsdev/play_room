import { writable } from 'svelte/store';
import type { PlayerId, RoomSnapshot } from '../protocol/types';

export const PARTICIPANT_SEAT_GRACE_MS = 90000;
export const SPECTATOR_NAME_GRACE_MS = 90000;

export interface SeatReservationsState {
  participantExpiresAt: Record<PlayerId, number>;
  spectatorExpiresAt: Record<PlayerId, number>;
}

function createSeatReservationsStore() {
  const { subscribe, set, update } = writable<SeatReservationsState>({
    participantExpiresAt: {},
    spectatorExpiresAt: {},
  });

  return {
    subscribe,
    markDisconnected: (playerId: PlayerId, atMs = Date.now()) => update((state) => ({
      participantExpiresAt: {
        ...state.participantExpiresAt,
        [playerId]: state.participantExpiresAt[playerId] ?? atMs + PARTICIPANT_SEAT_GRACE_MS,
      },
      spectatorExpiresAt: { ...state.spectatorExpiresAt },
    })),
    clearPlayer: (playerId: PlayerId) => update((state) => {
      const participantExpiresAt = { ...state.participantExpiresAt };
      const spectatorExpiresAt = { ...state.spectatorExpiresAt };
      delete participantExpiresAt[playerId];
      delete spectatorExpiresAt[playerId];
      return { participantExpiresAt, spectatorExpiresAt };
    }),
    syncRoom: (room: RoomSnapshot | null, nowMs = Date.now()) => update((state) => {
      if (!room) return { participantExpiresAt: {}, spectatorExpiresAt: {} };

      const participantExpiresAt: Record<PlayerId, number> = {};
      const spectatorExpiresAt: Record<PlayerId, number> = {};
      for (const player of room.players) {
        if (player.role === 'participant' && !player.connected) {
          participantExpiresAt[player.id] = player.participant_seat_expires_at_ms
            ?? state.participantExpiresAt[player.id]
            ?? nowMs + PARTICIPANT_SEAT_GRACE_MS;
        }
        if (player.role === 'spectator' && !player.connected) {
          spectatorExpiresAt[player.id] = player.spectator_expires_at_ms
            ?? state.spectatorExpiresAt[player.id]
            ?? nowMs + SPECTATOR_NAME_GRACE_MS;
        }
      }

      return { participantExpiresAt, spectatorExpiresAt };
    }),
    clear: () => set({ participantExpiresAt: {}, spectatorExpiresAt: {} }),
  };
}

export function reservationRemainingMs(expiresAt: number | undefined | null, nowMs = Date.now()): number | null {
  if (!expiresAt) return null;
  return Math.max(0, expiresAt - nowMs);
}

function countdownLabel(expiresAt: number | undefined | null, fallback: string, active: (seconds: number) => string, nowMs = Date.now()): string {
  const remainingMs = reservationRemainingMs(expiresAt, nowMs);
  if (remainingMs === null) return fallback;
  const seconds = Math.ceil(remainingMs / 1000);
  return seconds > 0 ? active(seconds) : 'Expiring';
}

export function reservedSeatLabel(expiresAt: number | undefined | null, nowMs = Date.now()): string {
  return countdownLabel(expiresAt, 'Seat reserved', (seconds) => `Seat reserved ${seconds}s`, nowMs);
}

export function reservedNameLabel(expiresAt: number | undefined | null, nowMs = Date.now()): string {
  return countdownLabel(expiresAt, 'Name reserved', (seconds) => `Name frees ${seconds}s`, nowMs);
}

export const seatReservationsStore = createSeatReservationsStore();