---
id: pw-ctx4
status: open
deps: [pw-ixzj]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: actix-web UI + API

Routes: GET / (index.html + seal.js), GET /api/scanner-pubkey (proxy scanner pubkey op), POST /api/init (forward sealed VK to scanner init, return walletAddress, start listener), GET /api/status. Form: paste viewing key; JS seals it and POSTs the ciphertext.

