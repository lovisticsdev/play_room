#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: scripts/run-server.sh [options]

Options:
  --config <path>     Server config file (default: examples/server.toml)
  --host <host>       Override configured host
  --port <port>       Override configured port
  -h, --help          Show this help
USAGE
}

config="examples/server.toml"
server_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config|-c)
      config="${2:?missing value for $1}"
      shift 2
      ;;
    --host|--port)
      server_args+=("$1" "${2:?missing value for $1}")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      server_args+=("$1")
      shift
      ;;
  esac
done

cd "$repo_root"
cargo run -p play-room-server -- --config "$config" "${server_args[@]}"
