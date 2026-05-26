#!/usr/bin/env bash
set -euo pipefail
NAME="${1:-player}"
cargo run -p play-room-client -- --name "$NAME"
