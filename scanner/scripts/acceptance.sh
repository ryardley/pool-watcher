#!/usr/bin/env bash
# acceptance.sh: end-to-end acceptance for the scanner.
#   1. Restart the chain so the full history is queryable.
#   2. eth_getLogs the proxy's Shield/Transact/Unshield logs for real.
#   3. Drive the scanner via the Rust handle and assert the events reconstruct
#      out/steps.jsonl and the balances arithmetic.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if [[ ! -f out/steps.jsonl ]]; then
  echo "error: out/steps.jsonl missing; run scripts/scenario.sh first" >&2
  exit 1
fi

# 1. Restart the chain (graceful stop dumps state, start reloads it).
if [[ -f .state/anvil.pid ]]; then
  scripts/stop-chain.sh
fi
scripts/chain.sh

# 2. Fetch the real logs from the chain into out/logs.json.
scripts/op.sh export-logs

# 3. Build the scanner and run the acceptance test.
cd scanner
npm run build
cargo test --test acceptance -- --nocapture
