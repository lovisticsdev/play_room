# Protocol

Play Room uses the same JSON request, response, event, and snapshot shapes across two transports.

- TCP clients send newline-delimited JSON. Each line is one complete JSON object.
- Browser clients send the same JSON objects in WebSocket text frames.

The server listens on one host/port and upgrades HTTP WebSocket handshakes while preserving the raw TCP path for the terminal client.

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
    "protocol_version": 1
  }
}
```

The `player_id` is the stable identity for the connected session. The `reconnect_token` is a private recovery credential. The browser should store it in tab-scoped session storage and attempt automatic reconnect after refresh or network loss in that tab.

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

A reconnect request with a valid token should restore the same player identity instead of creating a new player. If that player is still in a room, the client receives the current room snapshot and renders the existing room state.

Reconnect can fail when the token is unknown, the server restarted without session persistence, or the room/session no longer exists. The UI should then fall back to the connect and room browser flow.

## Room Updates

Room updates are broadcast as events and snapshots:

- events explain what just happened, such as joined, left, ready, move accepted, host changed, round resolved, game ended, or match reset
- snapshots are authoritative and should repair any stale local client state

Clients should use events for the live feed and snapshots for rendered truth.

Room summaries include player counts, spectator counts, game kind, and `target_score`, so clients can render labels such as Best of 3 or Best of 5.

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

The web client should branch on `code` and present `suggestions` as clickable alternatives when present.

## Naming Rules

Room names are unique server-wide, case-insensitively. Name-based join also resolves case-insensitively.

Player display names are unique within a room, including disconnected players who still own a reconnectable session. Reconnect restores the original player identity and bypasses name conflict because it is the same session.

When a duplicate display name is rejected, the server returns `player_name_exists`. If the existing player is disconnected, the message tells the user to reconnect with the session token or choose another name.