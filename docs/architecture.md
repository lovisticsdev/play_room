# Architecture

Play Room is split into crates by responsibility.

- `play-room-core` owns deterministic room and game state.
- `play-room-protocol` owns the network message schema and JSON codec.
- `play-room-server` owns TCP sockets, WebSocket upgrades, sessions, room registry, broadcast fanout, reconnect tokens, and timers.
- `play-room-client` owns terminal input/output and client-side connection handling.
- `play-room-testkit` owns scripted scenario data structures and test helpers.
- `web` owns the Svelte browser client and visual room/game state rendering.

The central rule is that socket handlers never mutate room internals directly. They convert client requests into core commands and apply those commands through the room state machine. The core returns events; the server broadcasts those events and snapshots.
