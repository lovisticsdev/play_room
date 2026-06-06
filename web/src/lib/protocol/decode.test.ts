import { describe, expect, it } from 'vitest';
import { decodeServerMessage, ProtocolDecodeError } from './decode';
import { PROTOCOL_VERSION } from './generated';
import type { GameRules } from './types';

const validRules: GameRules = {
  game: 'rock_paper_scissors_lizard_spock',
  min_players: 2,
  max_players: 2,
  target_score: 2,
  round_seconds: 15,
  allow_spectators: true,
};

function welcomeMessage(protocolVersion: number = PROTOCOL_VERSION) {
  return {
    kind: 'response',
    request_id: 1,
    result: {
      status: 'welcome',
      player_id: 'player-alice',
      display_name: 'Alice',
      reconnect_token: 'session-alice',
      protocol_version: protocolVersion,
      reconnected: false,
      stale_token_replaced: false,
      room_restored: false,
    },
  };
}

function roomSnapshotMessage(rules: GameRules = validRules) {
  return {
    kind: 'event',
    event: {
      type: 'room_snapshot',
      room: {
        id: 'room-testroom',
        name: 'testroom',
        host_id: 'player-alice',
        phase: { phase: 'lobby' },
        rules,
        round: 0,
        players: [
          {
            id: 'player-alice',
            name: 'alice',
            role: 'participant',
            ready: false,
            connected: true,
            score: 0,
            participant_seat_expires_at_ms: null,
            spectator_expires_at_ms: null,
          },
        ],
        scoreboard: [{ player_id: 'player-alice', name: 'alice', score: 0 }],
      },
    },
  };
}

function encode(message: unknown): string {
  return JSON.stringify(message);
}

describe('decodeServerMessage', () => {
  it('rejects malformed JSON', () => {
    expect(() => decodeServerMessage('{')).toThrow(ProtocolDecodeError);
  });

  it('rejects unknown server message kinds', () => {
    expect(() => decodeServerMessage(encode({ kind: 'bogus' }))).toThrow(ProtocolDecodeError);
  });

  it('rejects unsupported protocol versions', () => {
    expect(() => decodeServerMessage(encode(welcomeMessage(PROTOCOL_VERSION + 1)))).toThrow(
      /Unsupported protocol version/,
    );
  });

  it('rejects invalid room rules for the browser client', () => {
    expect(() =>
      decodeServerMessage(
        encode(
          roomSnapshotMessage({
            ...validRules,
            min_players: 3,
            max_players: 3,
          }),
        ),
      ),
    ).toThrow(/expects exactly 2 active participants/);
  });

  it('rejects unsafe uint64 request ids', () => {
    const message = welcomeMessage();
    message.request_id = Number.MAX_SAFE_INTEGER + 1;

    expect(() => decodeServerMessage(encode(message))).toThrow(ProtocolDecodeError);
  });

  it('rejects uint32 overflow in room snapshots', () => {
    const message = roomSnapshotMessage();
    message.event.room.round = 4_294_967_296;

    expect(() => decodeServerMessage(encode(message))).toThrow(ProtocolDecodeError);
  });

  it('rejects uint16 overflow in welcome protocol versions', () => {
    expect(() => decodeServerMessage(encode(welcomeMessage(65_536)))).toThrow(ProtocolDecodeError);
  });

  it('accepts max valid uint32 values where schema allows them', () => {
    const message = {
      kind: 'event',
      event: {
        type: 'room_event',
        room_id: 'room-testroom',
        event: {
          event: 'match_format_changed',
          target_score: 4_294_967_295,
        },
      },
    };

    expect(decodeServerMessage(encode(message))).toMatchObject(message);
  });

  it('rejects fractional integer fields', () => {
    const message = roomSnapshotMessage();
    (message.event.room as { phase: unknown }).phase = {
      phase: 'in_round',
      round: 1.5,
      deadline_ms: 2_000,
    };

    expect(() => decodeServerMessage(encode(message))).toThrow(ProtocolDecodeError);
  });

  it('accepts valid welcome responses', () => {
    const decoded = decodeServerMessage(encode(welcomeMessage()));

    expect(decoded).toMatchObject({
      kind: 'response',
      result: {
        status: 'welcome',
        protocol_version: PROTOCOL_VERSION,
      },
    });
  });

  it('accepts valid room snapshot events', () => {
    const decoded = decodeServerMessage(encode(roomSnapshotMessage()));

    expect(decoded).toMatchObject({
      kind: 'event',
      event: {
        type: 'room_snapshot',
        room: {
          id: 'room-testroom',
          rules: validRules,
        },
      },
    });
  });
});
