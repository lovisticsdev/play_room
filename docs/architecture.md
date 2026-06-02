# Architecture

Play Room is split into crates by responsibility so game rules stay deterministic, protocol types stay reusable, and transports remain replaceable.

## Crates

- `play-room-core` owns deterministic room and game state.
- `play-room-protocol` owns the network message schema and JSON codec.
- `play-room-server` owns TCP sockets, WebSocket upgrades, sessions, room registry, broadcast fanout, reconnect tokens, and timers.
- `play-room-client` owns terminal input/output and client-side connection handling. It is kept as a protocol/debug client, not the primary product surface.
- `play-room-testkit` owns scripted scenario data structures and test helpers.
- `web` owns the primary Svelte browser client and visual room/game state rendering.

## Data Flow

```text
client request
  -> protocol decode
  -> server router
  -> room manager
  -> core room command
  -> domain events
  -> broadcast events and authoritative snapshots
```

The central rule is that socket handlers never mutate room internals directly. They convert client requests into core commands and apply those commands through the room state machine. The core returns events; the server broadcasts those events and snapshots.

## Session Ownership

The server owns player sessions and reconnect tokens. A reconnect token restores the same player identity, which allows the user to return to the same room, score, role, and connected state when the server still has that session.

Room membership is tied to player identity, not the transport connection. A disconnected participant keeps their participant seat for 90 seconds so they can recover from a refresh or network drop without losing score or role. If they do not reconnect during that grace window, the server demotes them to a disconnected spectator, freeing the participant slot. Disconnected spectators then keep the room-scoped display name for another 90 seconds; if they still do not reconnect, the server removes them from the room and frees the name. The reconnect token remains valid for the player identity, but after room membership cleanup it no longer restores that room automatically.

## Room Ownership

Room names are unique server-wide, case-insensitively, so name-based joining is predictable. Display names are unique inside a room for current members, including disconnected members still inside their expiry window, so scoreboards, match notifications, and reconnect messages stay clear.

When a host leaves, the room promotes another remaining player, preferring connected participants. When the last player leaves, the room is removed from the registry.

## Match Lifecycle

Rooms default to Best of 3. A finished room keeps its final scoreboard and winner in the authoritative snapshot until the host starts the next match. Starting the next match resets scores, ready state, moves, and round number without changing current seats or spectators.

## Browser Direction

The browser client should now be treated as the main experience. Connection and room browsing should move into a modal/drawer, while the main screen focuses on the active room, game board, scoreboard, player/spectator lists, and transient match notifications.

See [web-ui-plan.md](web-ui-plan.md) for the planned UI structure.
