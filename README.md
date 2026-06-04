# Play Room

Play Room is a reconnectable real-time multiplayer room system for Rock Paper Scissors Lizard Spock. It pairs a Rust TCP/WebSocket backend with a Svelte browser client, a terminal protocol client, scripted scenario fixtures, and integration tests for the main gameplay flows.

The browser client is the primary user-facing experience. The terminal client remains as a lightweight protocol/debug client so the backend is not tied to one frontend.

## Features

- Shared server port for TCP JSON-lines clients and browser WebSocket clients
- Svelte + TypeScript web client for real-time room and game visualization
- Terminal client with room, ready, move, spectator, and reconnect commands
- Deterministic core room state machine
- Best of 3 default two-player match flow with timed rounds and timeout resolution
- Immediate forfeit resolution when an active participant leaves or disconnects mid-match
- Reconnect tokens with restored-room/stale-token metadata and stale socket-disconnect protection
- 30-second participant seat protection after disconnect, followed by 60-second spectator name cleanup
- Enforced room/client limits, abandoned session cleanup, and bounded outbound queues
- Unique room names, room-scoped display-name checks, and structured conflict suggestions
- Rust-generated browser protocol constants, TypeScript types, and JSON Schema validation
- Workspace integration tests and executable scripted fixtures

## Workspace Layout

```text
play_room/
|-- crates/
|   |-- play-room-core/      # deterministic game rules, room state, commands, events
|   |-- play-room-protocol/  # request/response DTOs, JSON codec, web protocol generation
|   |-- play-room-server/    # TCP/WebSocket server, sessions, rooms, timers, fanout
|   |-- play-room-client/    # terminal protocol/debug client
|   `-- play-room-testkit/   # scripted scenario and test helper utilities
|-- web/                     # primary Svelte browser client
|-- examples/                # server config and scripted client fixtures
|-- tests/                   # workspace integration tests
|-- Cargo.toml
|-- Cargo.lock
`-- README.md
```

## Requirements

- Rust stable toolchain
- Node.js 18+ for the browser client
- Cargo and npm available from your shell

## Run The App

Start the server:

```bash
cargo run -p play-room-server -- --config examples/server.toml
```

`cargo run -p play-room-server` builds and runs the `play-room-server` package. The `--` separates Cargo arguments from application arguments, so `--config examples/server.toml` is passed to the server itself.

The sample config listens on:

```text
127.0.0.1:7878
```

It also sets room/client quotas and abandoned-session cleanup:

```toml
host = "127.0.0.1"
port = 7878
max_rooms = 128
max_clients = 512
abandoned_session_ttl_seconds = 1800
```

The server uses `examples/server.toml` by default and falls back to built-in defaults if the file is missing. You can override host or port from the CLI:

```bash
cargo run -p play-room-server -- --host 127.0.0.1 --port 7878
```

Start the browser client in another shell:

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

Run terminal clients without helper wrappers:

```bash
cargo run -p play-room-client -- --name alice
cargo run -p play-room-client -- --name bob
```

With explicit server address:

```bash
cargo run -p play-room-client -- --name alice --host 127.0.0.1 --port 7878
```

With a reconnect token:

```bash
cargo run -p play-room-client -- --name alice --reconnect-token session-...
```

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

Useful terminal commands:

```text
/help                         show available commands
/rooms                        list active rooms
/create <room name>           create and join a room as host
/join <room_id|room_name>     join an existing room by ID or exact name
/leave                        leave the current room
/again | /next                reset a finished match as host
/ready                        mark yourself ready
/unready                      clear your ready state
/move <move>                  submit rock, paper, scissors, lizard, or spock
/spectate [room_id|room_name] join a room as spectator, or switch current role
/play                         switch current room role back to participant
/ping                         send a health check request
/quit                         disconnect the client
```

## Architecture

Play Room is split by responsibility so game rules stay deterministic, protocol types stay reusable, and transports remain replaceable.

- `play-room-core` owns deterministic room and game state. It has no sockets, async runtime, filesystem access, or protocol encoding.
- `play-room-protocol` owns the network message schema, JSON codec, protocol tag manifest, structural TypeScript type generation, JSON Schema generation, and web protocol generator.
- `play-room-server` owns TCP sockets, WebSocket upgrades, sessions, room registry, broadcast fanout, reconnect tokens, and timers.
- `play-room-client` owns terminal input/output and client-side connection handling.
- `play-room-testkit` owns scripted scenario data structures and test helpers.
- `web` owns the primary Svelte browser client and visual room/game state rendering.

Data flow:

```text
client request
  -> protocol decode
  -> server router
  -> room manager
  -> core room command
  -> domain events
  -> broadcast events and authoritative snapshots
```

Socket handlers never mutate room internals directly. They translate client requests into core commands, apply those commands through the room state machine, then broadcast events and snapshots.

The server keeps orchestration in `room_manager`, with focused helpers for `fanout`, `expiry`, `room_registry`, `session_registry`, `membership`, `room_lifecycle`, `scheduler`, `router`, `session`, and `websocket_session`.

## Match And Room Rules

Rooms move through:

```text
Lobby -> InRound -> Lobby -> ... -> Finished -> Lobby
```

Default rooms are Best of 3, represented as `target_score = 2`, meaning the first participant to two round wins takes the match. Supported competitive rooms are exactly two active participants because the RPS/RPSLS resolver compares one participant against one opponent.

Participants can join, leave, ready, unready, submit moves, and appear in the scoreboard. Spectators can watch room state but cannot submit moves and do not appear in competitive scores.

After each round resolves, participants return to not-ready so the next round requires an explicit ready check. A finished room keeps its final scoreboard and winner until the host sends `start_next_match`, which resets scores, ready state, moves, and round number without changing seats or spectators.

Room names are unique server-wide, case-insensitively. Display names are unique inside a room while a member is present, including disconnected members inside their expiry windows. Duplicate room/name errors include structured suggestions when alternatives are available.

When the host leaves or forfeits, host ownership transfers to another remaining player, preferring connected participants. When the last player leaves, the room is removed from the server registry.

## Disconnect And Reconnect

The server owns player sessions and reconnect tokens. A reconnect token restores the same player identity, allowing the user to return to the same room, score, role, and connected state while the server still retains that session.

Welcome responses expose reconnect outcome metadata:

- `reconnected`: a supplied token matched an existing identity
- `stale_token_replaced`: a supplied token was unknown and replaced with a fresh identity/token
- `room_restored`: the restored identity still had room membership and received a room snapshot

Disconnected participants keep their participant seat for 30 seconds. Reconnecting during that window restores them as active participants. After 30 seconds, the server demotes them to disconnected spectators and frees the participant slot. Disconnected spectators keep their room-scoped display name for 60 seconds. If they do not reconnect during that window, the server removes them from the room and frees the display name.

If an active participant leaves or disconnects after a match has started, the match ends by forfeit so the opponent is not trapped in a dead match. The grace periods protect identity and room clarity; they do not keep an abandoned match alive.

Each active socket has a server-side connection generation. If an older transport loop closes after a same-token reconnect, its stale disconnect is ignored so it cannot mark the restored socket offline. Each active socket also has a bounded outbound queue; if it fills or closes, the server drops that socket and normal reconnect handling can restore the player later.

## Protocol

The same JSON envelope is available through two transports:

- TCP clients send newline-delimited JSON.
- Browser clients send JSON in WebSocket text frames.

Each client request carries a numeric `request_id`; responses echo that ID. Room updates are sent as room events and authoritative snapshots. Events explain what happened, while snapshots are the rendered source of truth.

Move privacy is intentional: `move_accepted` identifies the player but does not reveal the selected move. Submitted moves are only exposed in the round result.

Rust protocol and core DTOs generate browser protocol artifacts:

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

Rust tests check that the committed generated constants, structural TypeScript types, and JSON Schema match Rust output. The browser validates incoming WebSocket frames with AJV before applying them to client state.

## Quality Checks

Before pushing, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For the browser client:

```bash
cd web
npm run generate:protocol
npm test
npm run check
npm run build
```

The Rust suite covers protocol round-trips, generated protocol drift, reconnect metadata, two-player matches, spectator restrictions, reconnect flow, stale socket-disconnect protection, timeout resolution, move privacy, disconnect expiry behavior, room/client limits, bounded queues, host transfer, empty-room cleanup, and every JSON fixture in `examples/scripted_clients/`.

The web suite covers runtime server-message decoding for malformed JSON, unknown message kinds, unsupported protocol versions, invalid room rules, and valid welcome/snapshot messages.

## Repository Notes

`Cargo.lock` is committed because this workspace includes runnable binaries. Web build output, local runtime files, logs, package installs, and editor metadata are ignored. Rust-generated browser protocol files under `web/src/lib/protocol/` are committed intentionally so the browser can type and validate server messages without a generation step at runtime.
