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

wallets.sh -> compose up -> paste B VK -> scenario.sh -> refresh Fava. bean-check ledger.beancount passes. Fava shows 5 sales + 1 withdrawal; Assets:Railgun:B = 185 TST; Income:Sales = -235; Assets:Bank:EOA2 = 50 - fee; Expenses:Fees = fee. Scanner tests pass with fee; pubkey + sealedbox round-trip works.

