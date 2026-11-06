---
id: pw-ixzj
status: open
deps: [pw-szlj]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: scanner client thread

Dedicated thread + mpsc channel so the sync railgun-scanner-handle is called from async actix/alloy code. Ops: pubkey(), init(keyCiphertext), scan(blocks), ping(). Wait for the socket to exist before connecting.

