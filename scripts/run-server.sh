#!/usr/bin/env bash
set -euo pipefail
cargo run -p play-room-server -- --config examples/server.toml "$@"
