#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <op> [flags]" >&2
  exit 1
fi
name="$1"
shift

if ! cast chain-id >/dev/null 2>&1; then
  echo "error: chain not running; run scripts/chain.sh" >&2
  exit 1
fi

export DB_PATH="${DB_PATH:-$ROOT/.state/db-testbed}"
export PW_CACHE_DIR="${PW_CACHE_DIR:-$ROOT/.cache}"
cd demo
exec npx tsx "src/ops/$name.ts" "$@"
