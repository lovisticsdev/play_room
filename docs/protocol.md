# Protocol

Play Room uses the same JSON request, response, event, and snapshot shapes across two transports.

- TCP clients send newline-delimited JSON. Each line is one complete JSON object.
- Browser clients send the same JSON objects in WebSocket text frames.

The server listens on one host/port and upgrades HTTP WebSocket handshakes while preserving the raw TCP path for the terminal client.

Client messages use this shape:

```json
{
  "request_id": 1,
  "request": { "type": "connect", "name": "alice", "reconnect_token": null }
}
```

Server messages use this shape:

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

Room updates are broadcast as events and snapshots. Clients should treat snapshots as authoritative.
