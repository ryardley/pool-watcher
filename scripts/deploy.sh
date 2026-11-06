#!/usr/bin/env bash
# deploy.sh: deploys the contracts to the local chain, records addresses and the
# deployment block in .state/deployment.json, and registers production vkeys.
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

deploy_start_block="$(cast block-number)"
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

# Anvil's --state reload does not keep historical state for eth_getCode, so a
# binary search for the exact proxy block fails after a restart. The block
# before deployment is a safe lower bound for scanning.
deployment_block="$deploy_start_block"

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
