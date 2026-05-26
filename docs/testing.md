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
