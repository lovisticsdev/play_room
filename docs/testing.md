# Testing

The test strategy focuses on invariants across the deterministic core, protocol codec, server room manager, and integration flows.

## Core Invariants

- RPS/RPSLS move comparison is deterministic.
- Default rooms are Best of 3 (`target_score = 2`).
- A room creator is joined atomically as host.
- Spectators cannot submit moves.
- Spectators are excluded from the competitive scoreboard.
- Disconnected participants remain visible when their session and score still matter.
- Duplicate display names are rejected within a room, including spectators and disconnected players.
- A ready room starts a round exactly once.
- Timeouts resolve active rounds and reject stale timeout commands.
- Finished matches carry the winner in `RoomPhase::Finished`.
- Host-only next-match reset clears scores without changing seats.

## Server And Protocol Invariants

- Protocol messages round-trip through JSON.
- Room names are unique server-wide, case-insensitively.
- Joining by room name is predictable and case-insensitive when names are unique.
- Duplicate room-name errors include suggested alternatives.
- Duplicate disconnected display-name errors are explicit enough for reconnect guidance.
- Moving between rooms preserves old-room leave events and snapshots.
- Reconnect restores the same player identity when the token is valid.
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

Planned web-facing coverage should add browser-store/unit checks for automatic reconnect, room-modal state, duplicate-name messaging, next-match controls, and scoreboard grouping.

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
npm run check
npm run build
```