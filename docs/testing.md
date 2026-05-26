# Testing

The test strategy focuses on invariants:

- RPS/RPSLS move comparison is deterministic.
- A room creator is joined atomically as host.
- Spectators cannot submit moves.
- A ready room starts a round exactly once.
- Timeouts resolve active rounds and reject stale timeout commands.
- Protocol messages round-trip through JSON.

Run all tests with:

```bash
cargo test --workspace
```
Scripted scenario fixtures under `examples/scripted_clients/` are executable examples. The `scripted_scenarios` integration test loads every JSON file, deserializes it through `play-room-testkit`, and runs it against an in-memory room state machine.