# Protocol

Play Room uses the same JSON request, response, event, and snapshot shapes across two transports.

- TCP clients send newline-delimited JSON. Each line is one complete JSON object.
- Browser clients send the same JSON objects in WebSocket text frames.

The server listens on one host/port and upgrades HTTP WebSocket handshakes while preserving the raw TCP path for the terminal client.

## Protocol Metadata

Rust protocol and core DTOs are the source for protocol tag values, structural browser types, and JSON Schema. The `play-room-protocol` crate serializes representative serde values into a tag manifest, emits structural TypeScript types, generates JSON Schema for client/server envelopes, and writes `web/src/lib/protocol/generated.ts`, `web/src/lib/protocol/generated-types.ts`, and `web/src/lib/protocol/schema.ts`.

Regenerate the browser protocol files with either command:

```bash
cargo run -p play-room-protocol --bin generate-web-protocol
```

```bash
cd web
npm run generate:protocol
```

`cargo test --workspace` includes drift tests that compare the checked-in generated constants, structural types, and schema with Rust-generated output. The web client imports generated constants/types and uses AJV with the generated server-message schema for runtime WebSocket validation. A small semantic guard still enforces domain constraints that JSON Schema does not express, such as currently supported two-player rules.

## Client Request

```json
{
  "request_id": 1,
  "request": {
    "type": "connect",
    "name": "alice",
    "reconnect_token": null
  }
}
```

Each client request carries a numeric `request_id`. Responses echo that ID so clients can match command results to user actions.

## Welcome Response

```json
{
  "kind": "response",
  "request_id": 1,
  "result": {
    "status": "welcome",
    "player_id": "player-...",
    "reconnect_token": "session-...",
    "protocol_version": 2,
    "reconnected": false,
    "stale_token_replaced": false,
    "room_restored": false
  }
}
```

The `player_id` is the stable identity for the connected session. The `reconnect_token` is a private recovery credential. The browser stores it in tab-scoped session storage and attempts automatic reconnect after refresh or network loss in that tab.

Welcome metadata makes reconnect outcomes explicit. `reconnected` means the supplied token matched an existing player identity. `stale_token_replaced` means a supplied token was unknown and the server issued a fresh identity/token. `room_restored` means the restored identity still has room membership and the server will send an authoritative room snapshot for it.

## Room And Match Requests

Important gameplay requests include:

```text
list_rooms
create_room
join_room
spectate_room
leave_room
set_ready
set_spectator
submit_move
start_next_match
```

`start_next_match` is valid only after the room is finished and only for the current host. It resets scores and ready state while keeping room membership intact.

## Reconnect Behavior

A reconnect request with a valid token restores the same player identity instead of creating a new player. The welcome response reports this with `reconnected: true`. If that player is still in a room, the response also reports `room_restored: true`, and the client receives the current room snapshot before rendering the restored room state.

Disconnected participants keep their participant seat for 30 seconds. Reconnecting during that window restores the player as an active participant. After the grace window, the server demotes the player to a disconnected spectator and frees the participant slot. The disconnected spectator then keeps the room-scoped display name for 60 seconds; reconnecting during that window restores the same identity as a spectator. If that second window expires, the server removes the disconnected spectator from the room and the display name becomes available again.

If the token is unknown, for example after a server restart without session persistence, the server treats it as stale: it creates a fresh player identity, returns a fresh reconnect token, sets `stale_token_replaced: true`, emits a notice, and does not restore room membership. During transient socket loss, the browser keeps the current room state visible, marks the connection as reconnecting, retries with a short backoff schedule, and only falls back to the connect/room browser flow after recovery fails or the welcome response confirms no room was restored.

## Room Updates

Room updates are broadcast as events and snapshots:

- events explain what just happened, such as joined, left, ready, move accepted, host changed, round resolved, game ended, or match reset. Move-accepted events intentionally identify the player but not the selected move; submitted moves are revealed only in the round result.
- snapshots are authoritative and repair stale local client state

Clients use events for the live feed and snapshots for rendered truth. Browser clients validate every incoming WebSocket frame against the generated server-message JSON Schema before applying it; malformed messages are treated as protocol errors instead of being trusted after JSON.parse.

Room summaries include player counts, spectator counts, game kind, and `target_score`, so clients can render labels such as Best of 3 or Best of 5.

Snapshot player views may include `participant_seat_expires_at_ms` for disconnected participants whose competitive seat is temporarily reserved, or `spectator_expires_at_ms` for disconnected spectators whose room-scoped display name is temporarily reserved. Browser clients render countdowns from these server deadlines instead of estimating them locally.

## Error Shape

Errors include a human message, optional machine-readable code, and optional suggestions:

```json
{
  "kind": "response",
  "request_id": 7,
  "result": {
    "status": "error",
    "message": "room name already exists: testroom",
    "code": "room_name_exists",
    "suggestions": ["testroom-2", "testroom-alice", "testroom-3"]
  }
}
```

The web client branches on `code` and presents `suggestions` as clickable alternatives when present. Quota failures use `room_limit_reached` for room-cap errors and `client_limit_reached` for retained-session-cap errors.

## Naming Rules

Room names are unique server-wide, case-insensitively. Name-based join also resolves case-insensitively.

Player display names are unique within a room while a member is present, including disconnected participants and disconnected spectators inside their expiry windows. After spectator name expiry removes the disconnected member from the room, the display name can be used again. Reconnect restores the original player identity and bypasses name conflict because it is the same session.

When a duplicate display name is rejected, the server returns `player_name_exists`. If the existing player is disconnected, the message tells the user to reconnect with the session token or choose another name.
