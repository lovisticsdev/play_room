# Play Room

Play Room is a reconnectable real-time multiplayer room system for Rock Paper Scissors Lizard Spock. It pairs a Rust TCP/WebSocket backend with a Svelte browser client, a terminal protocol client, scripted scenario fixtures, and integration tests for the main gameplay flows.

The browser client is the primary user-facing experience. The terminal client remains as a lightweight protocol/debug client so the backend is not tied to one frontend.

## Features

- Shared server port for TCP JSON-lines clients and browser WebSocket clients
- Svelte + TypeScript web client for real-time room and game visualization
- Terminal client with room, ready, move, spectator, and reconnect commands
- Deterministic core room state machine
- Participant-aware room flow with players and spectators
- Best of 3 default two-player match flow with timed rounds and timeout resolution
- Reconnect tokens with explicit restored-room and stale-token response metadata
- Enforced room/client limits with abandoned session cleanup
- Bounded outbound queues so stalled clients are dropped instead of growing memory
- 90-second participant seat protection after disconnect, followed by spectator demotion and timed name cleanup
- RPS and RPSLS move support
- Unique room names, room-scoped display-name checks, and structured conflict errors
- Rust-generated web protocol constants and JSON Schema for browser validation
- Workspace integration tests and executable scripted fixtures
- Warning-clean Rust checks with clippy warnings denied

## Browser Client

The Svelte client is the primary interface. It uses a Rooms modal for connection, reconnect, room browsing, room creation, joining, and spectating. Active room state is rendered through the match surface, participants/spectators rail, scoreboard, and transient notifications. Betting and money mechanics are not part of this project.

## Workspace Layout

```text
play_room/
|-- crates/
|   |-- play-room-core/      # deterministic game rules, room state, commands, events
|   |-- play-room-protocol/  # request, response, event, snapshot, and JSON codec types
|   |-- play-room-server/    # async TCP/WebSocket server, timers, sessions, room registry, lifecycle, expiry, fanout
|   |-- play-room-client/    # terminal protocol/debug client
|   `-- play-room-testkit/   # scripted scenario and test helper utilities
|-- web/                     # primary Svelte browser client
|-- docs/                    # architecture, protocol, state machine, and testing notes
|-- examples/                # server config and executable scripted client fixtures
|-- scripts/                 # convenience run scripts
|-- tests/                   # workspace integration tests
|-- Cargo.toml
|-- Cargo.lock
`-- README.md
```


## Requirements

- Rust stable toolchain
- Node.js 18+ for the browser client
- PowerShell, Bash, or any shell capable of running Cargo and npm commands

## Quick Start

Run the Rust test suite first:

```bash
cargo test --workspace
```

Start the server:

```bash
cargo run -p play-room-server -- --config examples/server.toml
```

The default server config listens on:

```text
127.0.0.1:7878
```

`examples/server.toml` also sets room/client quotas and the abandoned-session cleanup TTL:

```toml
max_rooms = 128
max_clients = 512
abandoned_session_ttl_seconds = 1800
```

Start the browser client:

```bash
cd web
npm install
npm run dev
```

Open:

```text
http://127.0.0.1:5173
```

The browser client connects to the server through WebSocket at:

```text
ws://127.0.0.1:7878/ws
```

Helper scripts are also available:

```powershell
.\scripts\run-server.ps1
.\scripts\run-web.ps1
.\scripts\run-client.ps1 -Name alice
.\scripts\run-client.ps1 -Name bob
```

```bash
bash scripts/run-server.sh
bash scripts/run-web.sh
bash scripts/run-client.sh alice
bash scripts/run-client.sh bob
```

See [scripts/README.md](scripts/README.md) for options.

## Terminal Walkthrough

In Alice:

```text
/create testroom
/ready
```

In Bob:

```text
/rooms
/join testroom
/ready
```

Then submit moves from each client:

```text
/move rock
/move scissors
```

The server broadcasts room events and authoritative snapshots as the match progresses. Outbound queues are bounded; a client that stops consuming server messages is dropped and can reconnect with its token. Competitive rooms are intentionally two-player, and rooms default to Best of 3. Room names are unique server-wide and display names are unique inside a room so reconnects, scores, and match notifications remain clear. Disconnected participants keep their seat for 90 seconds; after that they become disconnected spectators for another 90-second name-reservation window. If they still do not reconnect, the server removes them from the room and frees the display name. Move submissions are acknowledged without revealing the selected move until the round resolves.

## Client Commands

```text
/help                         show available commands
/rooms                        list active rooms
/create <room name>           create and join a room as host
/join <room_id|room_name>     join an existing room by ID or exact name
/leave                        leave the current room
/again | /next                 reset a finished match as host
/ready                        mark yourself ready
/unready                      clear your ready state
/move <move>                  submit rock, paper, scissors, lizard, or spock
/spectate [room_id|room_name] join a room as spectator, or switch current role
/play                         switch current room role back to participant
/ping                         send a health check request
/quit                         disconnect the client
```

## Protocol

The same JSON envelope is available through two transports:

- TCP clients send newline-delimited JSON.
- Browser clients send JSON in WebSocket text frames.

Each client request includes a numeric `request_id`; server messages are responses, room events, or room snapshots. Welcome responses explicitly report whether reconnect restored an identity, replaced a stale token, and restored room membership. Clients should treat snapshots as authoritative. The browser protocol constants in `web/src/lib/protocol/generated.ts` and JSON Schema in `web/src/lib/protocol/schema.ts` are generated from Rust serde DTO metadata. Browser WebSocket messages are validated with AJV before they reach application state.

See [docs/protocol.md](docs/protocol.md) for message examples and reconnect behavior.

## Architecture Notes

`play-room-core` has no sockets, async runtime, filesystem access, or protocol encoding. It accepts room commands and returns domain events. The server converts client requests into core commands, applies them through the state machine, and broadcasts events plus snapshots to connected clients.

More detail:

- [docs/architecture.md](docs/architecture.md)
- [docs/protocol.md](docs/protocol.md)
- [docs/state-machine.md](docs/state-machine.md)
- [docs/testing.md](docs/testing.md)

## Quality Checks

Before pushing, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For the web client, run:

```bash
cd web
npm run generate:protocol
npm run check
npm run build
```

The integration suite covers protocol round-trips, generated web protocol constant/schema drift, welcome reconnect metadata, two-player matches, spectator restrictions, reconnect flow, stale reconnect-token handling, timeout resolution, move privacy, disconnect expiry behavior, and every JSON fixture in `examples/scripted_clients/`.

## Repository Notes

`Cargo.lock` is committed because this workspace includes runnable binaries. Build output, local runtime files, logs, package installs, generated web output, and editor metadata are ignored.
