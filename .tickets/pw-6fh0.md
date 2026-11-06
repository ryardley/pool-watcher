---
id: pw-6fh0
status: closed
deps: []
links: []
created: 2026-11-06T15:45:27Z
type: task
priority: 2
tags: [scanner]
---
# Scanner: stateless trial-decryption service

## Description

The stateless Railgun trial-decryption service: one Unix socket per viewing
key, with a Rust handle (protocol + acceptance tests). See
`docs/scanner/spec.md`.

## Acceptance Criteria

- `init` → `ready`; `scan` (fixture) → `events`; `ping` → `pong`; error cases
  handled.
- Unit tests (Node) and protocol tests (Rust handle) pass.
- End-to-end acceptance (`scanner/scripts/acceptance.sh`) reconstructs
  `out/steps.jsonl` from real chain logs.

## References

- `docs/scanner/spec.md`

