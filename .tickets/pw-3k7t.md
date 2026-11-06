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

Merchant model: Assets:Railgun:B, Income:Sales, Assets:Bank:EOA2, Expenses:Fees, commodity TST (18 dp). Incoming transfer (transact, in, outputType=0) -> sale; unshield -> bank + fee; change notes (outputType=2) excluded. Dedup by txHash:logIndex:hash:direction; atomic temp+rename rewrite. Amounts: base units / 1e18.

