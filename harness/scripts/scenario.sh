#!/usr/bin/env bash
# scenario.sh: runs the end-to-end demo scenario on a fresh chain — setup, shield,
# a series of transfers, an unshield, and balance/keys reporting.
set -euo pipefail
cd "$(dirname "$0")"

if [[ -f ../out/steps.jsonl ]]; then
  echo "scenario already run on this chain; run scripts/reset.sh first" >&2
  exit 1
fi

./setup.sh

./op.sh shield --from-eoa 0 --to A --amount 1000

# Product-style memos: the accounting demo narrates these as merchant sales
# (wallet B is the merchant; see docs/app/spec.md).
memos=(
  "2x Espresso"
  "1x Latte, 1x Croissant, 1x Orange Juice"
  "2x Sandwich, 2x Coffee"
  "1x Cake, 3x Latte"
  "Catering: 10x Coffee, 10x Pastry"
)
i=0
for amt in 10 25 40 60 100; do
  flag=""; (( i % 2 == 0 )) && flag="--show-sender"
  ./op.sh transfer --from A --to B --amount "$amt" --memo "${memos[$i]}" $flag
  i=$((i+1))
done

./op.sh unshield --from B --to-eoa 2 --amount 50
./op.sh balances --wallets A,B --eoas 2
./op.sh export-keys
./op.sh export-logs
