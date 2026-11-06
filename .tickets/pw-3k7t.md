---
id: pw-3k7t
status: open
deps: [pw-4hz9, pw-szlj]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: Beancount merchant ledger writer

## Description

Turn the scanner's events for wallet B into a Beancount ledger that Fava can
serve. B is the merchant: the 5 incoming transfers are sales (product memos),
the unshield is a withdrawal to the bank, and the unshield fee is an expense.

## Design

Accounts and commodity:

- `Assets:Railgun:B` — B's private balance
- `Income:Sales` — customer payments
- `Assets:Bank:EOA2` — unshield destination
- `Expenses:Fees` — unshield fee
- Commodity `TST` (18 dp); amounts = base units ÷ 10¹⁸

Mapping (wallet B only):

| Scanner event | Beancount entry |
|---|---|
| `transact`, `in`, `outputType=0` | `Assets:Railgun:B +value` / `Income:Sales -value`; narration = product memo |
| `unshield` | `Assets:Railgun:B -value` / `Assets:Bank:EOA2 +(value-fee)` / `Expenses:Fees +fee`; narration `Withdrawal` |
| `transact`, `outputType=2` (change note) | excluded |
| `transact`, `out` (sent note) | excluded |

- Dedup key: `txHash:logIndex:hash:direction` (unshield:
  `txHash:logIndex:unshield`).
- Atomic rewrite: write `ledger.beancount.tmp`, then rename — Fava reloads on
  the rename. Restart = rebuild from backfill, so re-runs are idempotent.
- Header: `option "operating_currency" "TST"` + `open` directives for the four
  accounts.
- Metadata per entry: `txhash`, `block`, `logindex`, `wallet`, `token`.

## Acceptance Criteria

- Feeding the fixture events produces a balanced ledger (`bean-check` passes).
- 5 sales with product-memo narrations + 1 withdrawal with the fee split.
- `Assets:Railgun:B` = 185 TST; `Income:Sales` = −235; `Assets:Bank:EOA2` =
  50 − fee; `Expenses:Fees` = fee.
- Change notes are absent; re-running produces no duplicates.

## References

- `docs/app/spec.md` — Ledger writer + Event→Beancount mapping
- `scanner/src/decrypt.test.ts` — the expected B event set (values, memos)
- `docs/notes.md` — unshield value/fee semantics

