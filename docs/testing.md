# Testing

The test strategy focuses on invariants across the deterministic core, protocol codec, server room manager, and integration flows.

## Core Invariants

- RPS/RPSLS move comparison is deterministic.
- Default rooms are Best of 3 (`target_score = 2`).
- Supported RPS/RPSLS rooms validate as exactly two-player because the resolver compares one participant against one opponent.
- A room creator is joined atomically as host.
- Spectators cannot submit moves.
- Spectators are excluded from the competitive scoreboard.
- Disconnected participants remain visible when their session and score still matter.
- Disconnected participant seats are reserved for 30 seconds, then expire into disconnected spectators.
- Disconnected spectators reserve their display name for 60 seconds, then leave the room automatically so the name can be reused.
- Duplicate display names are rejected within a room, including spectators and disconnected players still inside their expiry windows.
- A ready room starts a round exactly once.
- Timeouts resolve active rounds and reject stale timeout commands.
- Finished matches carry the winner in `RoomPhase::Finished`.
- Host-only next-match reset clears scores without changing seats.

## Server And Protocol Invariants

- Protocol messages round-trip through JSON, including welcome reconnect metadata.
- Generated browser protocol constants, structural TypeScript types, and JSON Schema are checked against Rust-generated protocol output.
- Browser WebSocket messages are runtime-validated with AJV and generated JSON Schema before being applied to client state, with decoder unit tests for malformed and unsupported messages.
- Room names are unique server-wide, case-insensitively.
- Configured `max_rooms` rejects excess room creation.
- Configured `max_clients` rejects excess retained player identities while allowing reconnects to existing identities.
- Joining by room name is predictable and case-insensitive when names are unique.
- Duplicate room-name errors include suggested alternatives.
- Duplicate disconnected display-name errors are explicit enough for reconnect guidance.
- Moving between rooms preserves old-room leave events and snapshots.
- Reconnect restores the same player identity when the token is valid and reports whether room membership was restored.
- Stale disconnects from superseded same-token transports are ignored so an old socket cannot mark the active reconnect offline.
- Unknown reconnect tokens create a fresh identity, fresh token, explicit stale-token metadata, and explanatory notice.
- Reconnect before participant-seat expiry preserves the active seat; reconnect after participant-seat expiry but before spectator-name expiry restores the identity as a spectator.
- Expired spectator cleanup removes room membership and frees the display name.
- Abandoned disconnected sessions without room membership expire after the configured TTL and free client capacity.
- Bounded outbound queues report saturation and remove stalled active sockets.
- Host transfer and empty-room cleanup remain consistent after leave events.

## Integration And Scenario Coverage

Run all Rust tests with:

```bash
cargo test --workspace
```

Scripted scenario fixtures under `examples/scripted_clients/` are executable examples. The `scripted_scenarios` integration test loads every JSON file, deserializes it through `play-room-testkit`, and runs it against an in-memory room state machine.

Current high-value flows:

- protocol round-trip
- two-player Best of 3 match
- spectator flow
- reconnect flow
- timeout flow
- scripted JSON scenarios

The committed web checks are Vitest decoder tests, `svelte-check`, and a Vite production build. Browser automation and broader store-level unit tests are not part of the current suite.

## Manual Checks

Before pushing, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For the browser client:

```bash
cd web
npm test
npm run check
npm run build
```
