# State Machine

Rooms move through repeatable match phases:

```text
Lobby -> InRound -> Lobby -> ... -> Finished -> Lobby
```

The default rules are Best of 3, represented internally as `target_score = 2` or first player to two round wins. Best of 5 can still be represented by setting `target_score = 3` when a room is created.

## Lobby

Participants can join, leave, ready, unready, switch to spectator, or wait for another participant. A round starts when all connected participants are ready and the room has at least the configured minimum number of participants.

Spectators can watch room state but do not submit moves and should not appear in the competitive scoreboard.

## InRound

A round starts with a deadline. Participants submit one move each. A move becomes locked after it is accepted. The round resolves when all active participants submit moves or when the server schedules a timeout command after the deadline.

The server owns real timers. The core only stores deadlines and validates timeout commands. This keeps the core deterministic and unit-testable.

## Finished

When a participant reaches `target_score`, the room enters `Finished { winner }` and broadcasts `GameEnded`. Scores remain visible so clients can show the result banner and final scoreboard.

The room is not destroyed at match end. The host can send `StartNextMatch`, which resets scores, ready state, move state, and round number while keeping the current seats and spectators intact.

## Resolution

Resolution can produce:

- draw
- win by move comparison
- timeout win
- no-contest state when no valid winner exists

After each round resolution, participants return to not-ready so the next round requires an explicit ready check.

## Roles And Visibility

Participants are competitive players and appear in the scoreboard. Disconnected participants remain visible because their score and reconnect session still matter.

Spectators are non-competitive viewers. They should be grouped separately from participants in the UI and excluded from score totals.

## Host And Room Lifetime

The room creator starts as host. If the host leaves, host ownership transfers to another remaining player, preferring connected participants over spectators. If the last player leaves, the room is removed from the server registry.