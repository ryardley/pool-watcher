#!/usr/bin/env bash
# install.sh: idempotent one-time setup — creates test wallets, syncs the
# vendor/contract submodule, applies patches, and installs/compiles dependencies.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

for file in .wallet1 .wallet2; do
  if [[ -f "$file" ]]; then
    echo "skip: $file exists"
  else
    cast wallet new-mnemonic --color never |
      awk '/^Phrase:/{getline; print}' > "$file"
    echo "created: $file"
  fi
done

expected_submodule="$(git ls-files -s vendor/contract | awk '{print $2}')"
actual_submodule="$(git -C vendor/contract rev-parse HEAD 2>/dev/null || true)"
if [[ -n "$expected_submodule" && "$actual_submodule" == "$expected_submodule" ]]; then
  echo "skip: submodule at $expected_submodule"
else
  git submodule update --init --recursive
fi

PATCH="$ROOT/scripts/contracts-no-test-artifacts.patch"
if grep -q "test circuit artifacts are not installed" \
  vendor/contract/helpers/logic/artifacts.ts; then
  echo "skip: vendor/contract already patched"
else
  git -C vendor/contract apply "$PATCH"
  echo "patched: vendor/contract (test artifacts unavailable)"
fi

cd vendor/contract
lock_hash="$(sha256sum yarn.lock | awk '{print $1}')"
if [[ -d node_modules && -f node_modules/.installed &&
  "$(cat node_modules/.installed)" == "$lock_hash" ]]; then
  echo "skip: contracts dependencies installed"
else
  yarn install --frozen-lockfile --ignore-engines
  echo "$lock_hash" > node_modules/.installed
fi

if [[ -d artifacts && -f node_modules/.compiled &&
  "$(cat node_modules/.compiled)" == "$lock_hash" ]]; then
  echo "skip: contracts compiled"
else
  npx hardhat compile
  echo "$lock_hash" > node_modules/.compiled
fi

cd "$ROOT/demo"
lock_hash="$(sha256sum package-lock.json | awk '{print $1}')"
if [[ -d node_modules && -f node_modules/.installed &&
  "$(cat node_modules/.installed)" == "$lock_hash" ]]; then
  echo "skip: demo dependencies installed"
else
  npm ci --no-audit --no-fund
  echo "$lock_hash" > node_modules/.installed
fi
