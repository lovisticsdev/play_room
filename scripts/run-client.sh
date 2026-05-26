#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: scripts/run-client.sh [name] [options]

Options:
  --name <name>                 Client display name
  --host <host>                 Server host passed to play-room-client
  --port <port>                 Server port passed to play-room-client
  --reconnect-token <token>     Resume an existing session
  -h, --help                    Show this help
USAGE
}

name="player"
client_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --name|-n)
      name="${2:?missing value for $1}"
      shift 2
      ;;
    --host|--port|--reconnect-token)
      client_args+=("$1" "${2:?missing value for $1}")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --*)
      client_args+=("$1")
      shift
      ;;
    *)
      name="$1"
      shift
      ;;
  esac
done

cd "$repo_root"
cargo run -p play-room-client -- --name "$name" "${client_args[@]}"
