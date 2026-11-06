---
id: pw-bmv8
status: open
deps: [pw-fwdg, pw-fxzs, pw-sg82]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [verify]
---
# verify: end-to-end run + bean-check

## Description

The final gate: run the whole demo and prove the accounting pipeline works.
This is the acceptance test for the epic.

## Design

Run the demo flow:

1. `scripts/wallets.sh` — prints A/B addresses + viewing keys.
2. `docker compose up --build` (separate terminal).
3. Open `http://localhost:8080` — paste B's viewing key; scanner becomes
   ready, listener starts.
4. Open `http://localhost:5000` (Fava) — empty ledger.
5. `scripts/scenario.sh`.
6. Refresh Fava — 5 product-narrated sales + 1 withdrawal.

Then verify the ledger:

- `bean-check ledger.beancount` passes (double-entry balances).
- `Assets:Railgun:B` = 185 TST; `Income:Sales` = −235; `Assets:Bank:EOA2` =
  50 − fee; `Expenses:Fees` = fee.
- Scanner tests pass with the `fee` field; `pubkey` + `sealedbox` round-trip
  works.

## Acceptance Criteria

- The full demo flow completes without manual fixes.
- Fava shows exactly 5 sales (product memos) + 1 withdrawal.
- `bean-check` passes and the four account balances match the expected
  arithmetic.
- Re-running the flow after `scripts/reset.sh` works without re-downloading
  circuits or reinstalling packages.

## References

- `docs/app/spec.md` — Demo flow, Verification
- `docs/testbed/spec.md` — scenario, wallets.sh

