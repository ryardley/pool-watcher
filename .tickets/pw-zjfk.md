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

## Description

The scanner is a pure decrypt primitive with no RPC access — something must
hand it raw logs. The app's alloy listener fetches the proxy's
Shield/Transact/Unshield logs from Anvil and forwards them to the scanner in
`{ chainId, logs }` batches, then the ledger writer turns the returned events
into Beancount entries.

## Design

- Read `proxy` and `deploymentBlock` from `.state/deployment.json`
  (`config.rs`).
- **Backfill**: `eth_getLogs` from `deploymentBlock` to latest, filtered by
  the proxy address.
- **Subscribe**: WS `eth_subscribe` to new logs, filtered by the proxy
  address (alloy `subscribe_logs`).
- Group logs by block into `BlockLogs { chainId, logs }` batches and forward
  to the scanner via the scanner client thread.
- Fetch each block's timestamp (`eth_getBlockByNumber`) for the Beancount
  date.
- Starts after the scanner is initialized (the `POST /api/init` handler).
- The scanner ignores non-Railgun logs, so filtering by proxy address alone is
  sufficient.

## Acceptance Criteria

- Backfill returns the full history from `deploymentBlock` to head.
- Live subscription delivers new blocks as the scenario runs.
- Logs are batched by block and forwarded to the scanner; events come back.
- Block timestamps are available for the ledger writer.

## References

- `docs/app/spec.md` — Alloy listener section
- `demo/src/ops/export-logs.ts` — reference for the log filtering/batching
  shape
- `scanner/crates/handle/src/lib.rs` — `BlockLogs`/`Scan` wire structs

