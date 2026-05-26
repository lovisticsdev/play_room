#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
web_root="$repo_root/web"

usage() {
  cat <<'USAGE'
Usage: scripts/run-web.sh [options]

Options:
  --host <host>     Vite host (default: 127.0.0.1)
  --port <port>     Vite port (default: 5173)
  --install         Run npm install before starting Vite
  -h, --help        Show this help
USAGE
}

host="127.0.0.1"
port="5173"
install=0
extra_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      host="${2:?missing value for $1}"
      shift 2
      ;;
    --port)
      port="${2:?missing value for $1}"
      shift 2
      ;;
    --install)
      install=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      extra_args+=("$1")
      shift
      ;;
  esac
done

cd "$web_root"
if [[ "$install" == "1" || ! -d node_modules ]]; then
  npm install
fi

npm run dev -- --host "$host" --port "$port" "${extra_args[@]}"
