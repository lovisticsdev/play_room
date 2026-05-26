# Play Room

Play Room is a Rust workspace for a reconnectable multiplayer game room system. It includes deterministic Rock Paper Scissors / Rock Paper Scissors Lizard Spock game logic, a JSON-lines TCP protocol, an async server, a terminal client, scripted test helpers, and integration tests for the main gameplay flows.

The project is split by responsibility so the game rules stay deterministic and testable while the server owns sockets, sessions, broadcasts, reconnect tokens, and timers.

## Features

- TCP server using newline-delimited JSON messages
- Terminal client with room, ready, move, spectator, and reconnect commands
- Deterministic core room state machine
- Participant-aware room flow with players and spectators
- Timed rounds and timeout resolution
- Reconnect tokens for client sessions
- RPS and RPSLS move support
- Workspace integration tests and scripted test helpers
- Warning-clean Rust checks with clippy warnings denied

## Workspace Layout

```text
play_room/
|-- crates/
|   |-- play-room-core/      # Pure game rules, room state, commands, events
|   |-- play-room-protocol/  # JSON-lines protocol messages and codecs
|   |-- play-room-server/    # Async TCP server, sessions, timers, broadcasts
|   |-- play-room-client/    # Terminal client runtime and command parser
|   `-- play-room-testkit/   # Scripted scenario and test helper utilities
|-- docs/                    # Architecture, protocol, state-machine, testing notes
|-- examples/                # Server config and executable scripted client fixtures
|-- scripts/                 # Convenience run scripts
|-- tests/                   # Workspace integration tests
|   `-- common/              # Shared integration-test helpers
|-- Cargo.toml
|-- Cargo.lock
`-- README.md
```

## Requirements

- Rust stable toolchain
- PowerShell, Bash, or any shell capable of running Cargo commands

## Quick Start

Run the full test suite first:

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

Start two clients in separate terminals:

```bash
cargo run -p play-room-client -- --name alice
cargo run -p play-room-client -- --name bob
```

PowerShell helper scripts are also available:

```powershell

.\scripts\run-server.ps1
.\scripts\run-client.ps1 -Name alice
.\scripts\run-client.ps1 -Name bob
```

## Gameplay Walkthrough

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

The server broadcasts room events and authoritative snapshots as the match progresses. Room names are accepted when they are unique; if names collide, use the generated room ID from `/rooms`.

## Client Commands

```text
/help                       show available commands
/rooms                      list active rooms
/create <room name>         create and join a room as host
/join <room_id|room_name>   join an existing room by ID or exact name
/leave                      leave the current room
/ready                      mark yourself ready
/unready                    clear your ready state
/move <move>                submit rock, paper, scissors, lizard, or spock
/spectate [room_id|room_name] join a room as spectator, or switch current role
/play                       switch current room role back to participant
/ping                       send a health check request
/quit                       disconnect the client
```

## Protocol

The wire protocol is newline-delimited JSON over TCP. Each client request includes a numeric `request_id`; server messages are responses, room events, or room snapshots. Clients should treat snapshots as authoritative.

See [docs/protocol.md](docs/protocol.md) for message examples.

## Architecture Notes

`play-room-core` has no sockets, async runtime, filesystem access, or protocol encoding. It accepts room commands and returns domain events. The server converts client requests into core commands, applies them through the state machine, and broadcasts events plus snapshots to connected clients.

More detail:

- [docs/architecture.md](docs/architecture.md)
- [docs/state-machine.md](docs/state-machine.md)
- [docs/testing.md](docs/testing.md)

## Quality Checks

Before pushing, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The integration suite covers protocol round-trips, two-player matches, spectator restrictions, reconnect flow, timeout resolution, and every JSON fixture in `examples/scripted_clients/`.

## Repository Notes

`Cargo.lock` is committed because this workspace includes runnable binaries. Build output, local runtime files, logs, and editor metadata are ignored.
