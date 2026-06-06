import Ajv, { type ErrorObject, type ValidateFunction } from 'ajv';
import { PROTOCOL_VERSION } from './generated';
import { RACE_TARGETS } from './rules';
import { SERVER_MESSAGE_SCHEMA } from './schema';
import type { RoomSnapshot, ServerMessage, ServerResult } from './types';

const ajv = new Ajv({ allErrors: true, strict: false });
addRustIntegerFormats(ajv);
const validateServerMessage = ajv.compile(SERVER_MESSAGE_SCHEMA as object) as ValidateFunction;

export class ProtocolDecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ProtocolDecodeError';
  }
}

export function decodeServerMessage(raw: unknown): ServerMessage {
  let parsed: unknown;
  try {
    parsed = JSON.parse(String(raw));
  } catch {
    throw new ProtocolDecodeError('Server message is not valid JSON.');
  }

  if (!validateServerMessage(parsed)) {
    throw new ProtocolDecodeError(formatValidationErrors(validateServerMessage.errors));
  }

  const message = parsed as ServerMessage;
  assertSupportedSemantics(message);
  return message;
}

function addRustIntegerFormats(instance: Ajv): void {
  instance.addFormat('uint', {
    type: 'number',
    validate: isSafeUnsignedInteger,
  });
  instance.addFormat('uint16', {
    type: 'number',
    validate: (value: number) => isSafeUnsignedInteger(value) && value <= 65_535,
  });
  instance.addFormat('uint32', {
    type: 'number',
    validate: (value: number) => isSafeUnsignedInteger(value) && value <= 4_294_967_295,
  });
  instance.addFormat('uint64', {
    type: 'number',
    validate: isSafeUnsignedInteger,
  });
}

function isSafeUnsignedInteger(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0;
}

function formatValidationErrors(errors: ErrorObject[] | null | undefined): string {
  if (!errors?.length) {
    return 'Server message does not match the Play Room protocol schema.';
  }

  const details = errors
    .slice(0, 4)
    .map((error) => {
      const path = error.instancePath || '/';
      return `${path} ${error.message ?? 'is invalid'}`;
    })
    .join('; ');

  return `Server message does not match the Play Room protocol schema: ${details}`;
}

function assertSupportedSemantics(message: ServerMessage): void {
  if (message.kind === 'response') {
    assertSupportedResult(message.result);
    return;
  }

  const event = message.event;
  if (event.type === 'room_snapshot') {
    assertSupportedRoomSnapshot(event.room);
  }
}

function assertSupportedResult(result: ServerResult): void {
  switch (result.status) {
    case 'welcome':
      if (result.protocol_version !== PROTOCOL_VERSION) {
        throw new ProtocolDecodeError(
          `Unsupported protocol version: ${result.protocol_version}; expected ${PROTOCOL_VERSION}.`,
        );
      }
      return;
    case 'room_snapshot':
      assertSupportedRoomSnapshot(result.room);
      return;
    default:
      return;
  }
}

function assertSupportedRoomSnapshot(room: RoomSnapshot): void {
  const { rules } = room;
  if (rules.min_players !== 2 || rules.max_players !== 2) {
    throw new ProtocolDecodeError('Unsupported room rules: browser client expects exactly 2 active participants.');
  }

  if (!RACE_TARGETS.some((target) => target === rules.target_score)) {
    throw new ProtocolDecodeError('Unsupported room rules: target_score must be one of 1, 2, or 3.');
  }

  if (rules.round_seconds < 1) {
    throw new ProtocolDecodeError('Unsupported room rules: round_seconds must be at least 1.');
  }
}
