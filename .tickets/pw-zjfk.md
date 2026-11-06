---
id: pw-zjfk
status: open
deps: [pw-ixzj]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: alloy listener

Read proxy + deploymentBlock from .state/deployment.json. Backfill eth_getLogs from deploymentBlock to latest, then WS eth_subscribe for new logs, filtered by proxy address. Group logs by block into {chainId, logs} batches, forward to the scanner, fetch block timestamps for Beancount dates. Starts after scanner init.

