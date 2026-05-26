# Play Room Web Client

Svelte + TypeScript browser client for Play Room. It connects to the Rust server over WebSocket and reuses the same request, response, event, and snapshot message shapes as `play-room-protocol`.

Default WebSocket URL:

```text
ws://127.0.0.1:7878/ws
```

## Run

Start the Rust server from the repository root:

```bash
cargo run -p play-room-server -- --config examples/server.toml
```

Then start the web client:

```bash
cd web
npm install
npm run check
npm run build
npm run dev
```

Open:

```text
http://127.0.0.1:5173
```

## UI Coverage

- Connect/disconnect with display name and optional reconnect token
- Room list refresh
- Create room
- Join by room ID or unique room name
- Spectate by room ID or unique room name
- Ready/unready
- Submit RPS/RPSLS moves
- Switch between participant/spectator role
- Leave room
- Scoreboard, participants, spectators, round phase, timer, and event log
