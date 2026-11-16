#!/usr/bin/env bash
# reset.sh: tears down the local chain and wipes all .state and out artifacts
# so the testbed can be rebuilt from scratch.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/stop-chain.sh
rm -rf .state out
echo "reset: removed .state and out"
