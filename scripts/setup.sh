#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/install.sh
./scripts/chain.sh
./scripts/deploy.sh
./scripts/op.sh create-wallets --names A,B
./scripts/op.sh fund --eoa 0 --amount 1000
