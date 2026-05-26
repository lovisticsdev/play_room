# State machine

Rooms move through these phases:

```text
Lobby -> InRound -> Lobby -> ... -> Finished
```

A round starts when all connected participants are ready and the room has at least the configured minimum number of participants. A round resolves when all active participants submit moves or when the server schedules a timeout command after the deadline.

The server owns real timers. The core only stores deadlines and validates timeout commands. This keeps the core deterministic and unit-testable.
