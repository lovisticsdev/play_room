# Protocol

The protocol is newline-delimited JSON over TCP. Each line is one complete JSON object.

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
