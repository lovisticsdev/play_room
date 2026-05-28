# Web UI Plan

The browser client is the primary user-facing experience for Play Room. The terminal client stays in the workspace as a protocol/debug client, but product polish should now happen in `web/`.

## Direction

Play Room should feel like a live arcade command room for RPSLS:

- no betting or money mechanics
- active match state first
- connection and room browsing treated as setup/navigation
- spectators separated from competitive participants
- reconnect handled quietly by the client, with manual recovery available

## Target Screen Model

The main app screen should be dedicated to the current room:

```text
Top bar
  logo | Rooms button | connection status | player identity | reconnect token action

Main match area
  room name/code | phase | countdown | game board | move state | result state

Right rail
  participants with scores | spectators | event log | leave/settings
```

Connection, reconnect, create room, join room, and room browsing should move into a modal or drawer opened from the `Rooms` button. That keeps room discovery from competing with active play.

## Rooms Modal

The modal should support three states:

- not connected: connect form plus optional reconnect token
- connected without a room: room browser and create-room flow
- connected inside a room: room browser for switching/watching, with the current room marked

Room rows should use contextual actions:

- `Join` when a participant slot is available
- `Watch` when the room is full or already in progress
- `Current` for the user's current room
- `Join by Code` as a secondary/manual option

Duplicate room names should be rejected server-side and surfaced in the UI with suggested alternatives, not silently renamed.

## Match Area

The match area should show:

- room name and copyable room code/id
- current phase: lobby, waiting, round active, move locked, resolved, timeout
- countdown timer during active rounds
- five move buttons: rock, paper, scissors, lizard, spock
- disabled/locked move state after submission
- human-readable result banner after resolution

The game board and scoreboard should remain visible without scrolling during normal desktop play.

## Players And Scores

The UI should separate identity from scoring:

- Participants appear in the scoreboard and player panel.
- Spectators appear in a separate spectator group.
- Disconnected participants stay visible with a `Disconnected` badge because their score and reconnect session still matter.
- Spectators should not be shown as score-zero competitors.

Recommended naming policy:

- room names are unique server-wide, case-insensitively
- display names are unique within a room, including disconnected players
- reconnecting with a token restores the original identity and bypasses display-name conflict checks because it is the same session

## Reconnect UX

The browser should store the reconnect token in tab-scoped session storage after a successful connect. On refresh in that tab, it should attempt automatic reconnect before showing a fresh connect flow. A second browser tab should start as a separate player unless the user manually enters a token.

Manual reconnect token entry should remain available in the modal/session area for testing and recovery.

## Implementation Phases

1. Convert connection and room browsing into a modal/drawer.
2. Make the active room the permanent main surface.
3. Rebuild the right rail around participants, spectators, and event log.
4. Add local reconnect-token persistence and automatic reconnect.
5. Add room-name/player-name conflict messaging with suggested alternatives.
6. Polish game-state presentation: move locked, timeout, result, host transfer, disconnected player.

