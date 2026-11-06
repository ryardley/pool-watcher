#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p .state

if [[ -f .state/deployment.json ]]; then
  proxy="$(jq -r .proxy .state/deployment.json)"
  if [[ -n "$(cast code "$proxy" 2>/dev/null)" &&
    "$(cast code "$proxy" 2>/dev/null)" != "0x" ]]; then
    echo "skip: deployment already present at $proxy"
    exit 0
  fi
fi

rm -f .state/wallets.json
rm -rf .state/db-* out/

echo "deploying contracts..."
(cd vendor/contract && npx hardhat deploy:test --network localhost) \
  2>&1 | tee .state/deploy.log

get_addr() {
  sed -n "s/^  $1: '\(0x[0-9a-fA-F]*\)'.*/\1/p" .state/deploy.log | tail -1
}

proxy="$(get_addr proxy)"
delegator="$(get_addr delegator)"
testERC20="$(get_addr testERC20)"
weth9="$(get_addr weth9)"
relayAdapt="$(get_addr relayAdapt)"
if [[ -z "$proxy" || -z "$delegator" || -z "$testERC20" ]]; then
  echo "error: could not parse deployment addresses" >&2
  exit 1
fi

lo=0
hi="$(cast block-number)"
while [[ "$lo" -lt "$hi" ]]; do
  mid=$(((lo + hi) / 2))
  if [[ "$(cast code "$proxy" --block "$mid")" != "0x" ]]; then
    hi="$mid"
  else
    lo=$((mid + 1))
  fi
done
deployment_block="$lo"

jq -n \
  --arg proxy "$proxy" \
  --arg delegator "$delegator" \
  --arg testERC20 "$testERC20" \
  --arg weth9 "$weth9" \
  --arg relayAdapt "$relayAdapt" \
  --argjson deploymentBlock "$deployment_block" \
  '{proxy:$proxy, delegator:$delegator, testERC20:$testERC20, weth9:$weth9, relayAdapt:$relayAdapt, deploymentBlock:$deploymentBlock}' \
  > .state/deployment.json

echo "deployment saved:"
cat .state/deployment.json

echo "registering production vkeys..."
(cd demo && npx tsx src/register-vkeys.ts)
