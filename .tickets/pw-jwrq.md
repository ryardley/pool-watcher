---
id: pw-jwrq
status: open
deps: [pw-z7gq, pw-ctx4]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: browser seal bundle + static assets

app/web/seal.ts + package.json (noble deps, esbuild) built into app/static/seal.js. Serves index.html. Seals the pasted VK to the scanner pubkey per the sealed-box envelope.

