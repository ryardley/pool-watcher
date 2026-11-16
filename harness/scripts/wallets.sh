#!/usr/bin/env bash
# wallets.sh: the accounting demo's first step — full setup (install, chain,
# deploy, create wallets A/B), then export and print each wallet's address and
# shareable viewing key from out/keys.json.
set -euo pipefail
cd "$(dirname "$0")"

./install.sh
./chain.sh
./deploy.sh
./op.sh create-wallets --names A,B
./op.sh export-keys

jq -r 'to_entries[] | "\(.key)  \(.value.railgunAddress)\n  viewing key: \(.value.shareableViewingKey)\n"' ../out/keys.json
