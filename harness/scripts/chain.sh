#!/usr/bin/env bash
# chain.sh: starts the local anvil chain (with state persistence) if it is not
# already running, and records the PID in .state/anvil.pid.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p .state

if cast chain-id >/dev/null 2>&1 && [[ "$(cast chain-id)" == "31337" ]]; then
  echo "skip: chain already running on 31337"
  exit 0
fi

anvil --state .state/anvil.json > .state/anvil.log 2>&1 &
echo $! > .state/anvil.pid

for _ in $(seq 1 50); do
  if ! kill -0 "$(cat .state/anvil.pid)" 2>/dev/null; then
    echo "error: anvil exited; see .state/anvil.log" >&2
    cat .state/anvil.log >&2
    exit 1
  fi
  if cast chain-id >/dev/null 2>&1; then
    echo "chain started (pid $(cat .state/anvil.pid))"
    exit 0
  fi
  sleep 0.2
done

echo "error: anvil did not start; see .state/anvil.log" >&2
exit 1
