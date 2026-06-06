# Play Room

Play Room is a real-time multiplayer Rock Paper Scissors / Rock Paper Scissors Lizard Spock room game. It pairs a Rust backend with a Svelte + TypeScript browser client, reconnectable sessions, generated protocol artifacts, and automated tests for the main multiplayer flows.

The browser client is the primary experience. A terminal client is also included for protocol-level play and manual testing.

## Highlights

- Rust TCP/WebSocket server with shared room state
- Svelte + TypeScript browser client for live room and match visualization
- Deterministic core game state machine with no transport dependencies
- Race to 1, 2, or 3 match format, configurable by the current room host
- Timed rounds, private move submission, timeout handling, and forfeit resolution
- Reconnect tokens with stale-token metadata and session takeover protection
- Participant seat grace period and spectator name cleanup after disconnects
- Room-scoped display-name checks and unique room names with suggestions
- Bounded outbound queues, room/client limits, and abandoned-session cleanup
- Rust-generated TypeScript protocol types, constants, and JSON Schema
- Workspace integration tests, protocol drift tests, web decoder tests, and scripted scenario fixtures

## Project Layout

```text
play_room/
|-- crates/
|   |-- play-room-core/      # game rules, room state, commands, events
|   |-- play-room-protocol/  # DTOs, JSON codec, schema/type generation
|   |-- play-room-server/    # TCP/WebSocket server, sessions, rooms, timers
|   |-- play-room-client/    # terminal client
|   `-- play-room-testkit/   # scripted scenarios and test helpers
|-- web/                     # Svelte browser client
|-- examples/                # server config and scripted room flows
|-- tests/                   # workspace integration tests
|-- Cargo.toml
|-- Cargo.lock
`-- README.md
```

## Requirements

- Rust stable
- Node.js 18+
- Cargo and npm available from your shell

## Running Locally

Start the server:

```bash
cargo run -p play-room-server -- --config examples/server.toml
```

The sample config listens on `127.0.0.1:7878` and defines room/client limits:

```toml
host = "127.0.0.1"
port = 7878
max_rooms = 128
max_clients = 512
abandoned_session_ttl_seconds = 1800
```

Start the web client in another shell:

```bash
cd web
npm install
npm run dev
```

Open:

```text
http://127.0.0.1:5173
```

The browser connects to:

```text
ws://127.0.0.1:7878/ws
```

## Terminal Client

Run two terminal clients:

```bash
cargo run -p play-room-client -- --name alice
cargo run -p play-room-client -- --name bob
```

Common commands:

```text
/rooms                        list active rooms
/create <room name>           create and join a room as host
/join <room_id|room_name>     join an existing room
/spectate [room_id|room_name] watch a room or switch to spectator
/play                         switch back to participant when a seat is available
/race <1|2|3>                 change the room's race target as host
/ready | /unready             update ready state
/move <move>                  submit rock, paper, scissors, lizard, or spock
/name <display name>          update display name
/again | /next                reset a finished match as host
/leave                        leave the current room
/quit                         disconnect
```

## Gameplay Model

Rooms support exactly two active participants. Additional users can join as spectators.

Each room uses a Race to N format, where N is 1, 2, or 3. The default is Race to 2. The current host can change the target before a match starts or after it finishes.

Participants ready up to start each round. Moves are accepted privately and revealed only when the round resolves. Draws and no-contests do not award points, so play continues until a participant reaches the room target.

If an active participant leaves or disconnects after a match has started, the match resolves by forfeit. Host ownership transfers to another remaining player, preferring connected participants. Empty rooms are removed.

## Reconnect Behavior

Each connected player receives a reconnect token. Reusing that token restores the same player identity, room, score, role, and connected state while the server still retains the session.

Disconnected participants keep their seat for 30 seconds. After that, they become disconnected spectators and the participant slot is freed. Disconnected spectators keep their room-scoped display name for 60 seconds before cleanup.

If the same reconnect token is used in another tab or client, the previous socket receives a session-replaced event and stops reconnecting. This prevents reconnect loops between browser tabs.

## Architecture

Play Room separates deterministic game logic from transport and UI concerns:

- `play-room-core` owns room state, game rules, commands, events, scoring, and timers.
- `play-room-protocol` owns JSON message types, protocol versioning, encoding, generated TypeScript, and JSON Schema.
- `play-room-server` owns TCP/WebSocket sessions, room registry, reconnect tokens, fanout, expiry scheduling, and capacity limits.
- `play-room-client` owns terminal input/output.
- `play-room-testkit` owns scripted scenarios and integration-test helpers.
- `web` owns browser state, runtime protocol validation, and Svelte UI rendering.

Request flow:

```text
client request
  -> protocol decode
  -> server router
  -> room manager
  -> core room command
  -> domain events
  -> room events and authoritative snapshots
```

Room events describe what changed. Room snapshots are the rendered source of truth.

## Protocol Artifacts

Rust is the source of truth for protocol constants, structural TypeScript types, and JSON Schema. Generated files are committed under:

```text
web/src/lib/protocol/generated.ts
web/src/lib/protocol/generated-types.ts
web/src/lib/protocol/schema.ts
```

Regenerate them with:

```bash
cargo run -p play-room-protocol --bin generate-web-protocol
```

or:

```bash
cd web
npm run generate:protocol
```

The web client validates incoming WebSocket messages with AJV before applying them to client state.

## Quality Checks

Rust:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Web:

```bash
cd web
npm run generate:protocol
npm test
npm run check
npm run build
```

The test suite covers protocol round-trips, generated-artifact drift, room entry, two-player matches, reconnect flow, session takeover, timeout resolution, move privacy, spectator restrictions, disconnect expiry, room/client limits, bounded queues, host transfer, and scripted scenario fixtures.

## Repository Notes

`Cargo.lock` is committed because the workspace includes runnable binaries. Build output, local runtime files, logs, package installs, and editor metadata are ignored. Generated protocol files are committed intentionally so the web client can type-check and validate protocol messages without generating files at runtime.
