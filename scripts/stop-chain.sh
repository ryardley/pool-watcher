#!/usr/bin/env bash
# stop-chain.sh: gracefully stops the local anvil chain and cleans up its PID file.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ ! -f .state/anvil.pid ]]; then
  echo "skip: no .state/anvil.pid"
  exit 0
fi

pid="$(cat .state/anvil.pid)"
if kill -0 "$pid" 2>/dev/null; then
  kill -INT "$pid"
  for _ in $(seq 1 50); do
    if ! kill -0 "$pid" 2>/dev/null; then
      break
    fi
    sleep 0.1
  done
  echo "stopped: anvil (pid $pid)"
else
  echo "skip: anvil (pid $pid) not running"
fi
rm -f .state/anvil.pid
