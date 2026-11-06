---
id: pw-fxzs
status: open
deps: [pw-fwdg]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 2
parent: pw-mn26
tags: [docs]
---
# docs: write docs/app/spec.md

## Description

The app's reference spec: architecture, sealed-box envelope, event→Beancount
mapping, demo flow, and verification. A draft already exists (written as
context); verify it matches the implementation and fill any gaps.

## Design

`docs/app/spec.md` should cover:

- Goal + architecture diagram (host testbed → compose: scanner-b, app, fava)
- Locked decisions (Beancount/Fava, wallet B, alloy backfill+subscribe,
  merchant ledger, sealed box, compose topology)
- Repo layout (`app/` crate)
- Sealed-box envelope (exact wire format + KDF/AEAD)
- Web UI / API routes
- Scanner client, alloy listener, Beancount ledger writer
- Event → Beancount mapping table
- Config/env table
- Docker/compose services
- Demo flow (the 7 steps)
- Verification + testing

## Acceptance Criteria

- `docs/app/spec.md` exists and matches the implemented `app/` behavior.
- It is linked from `docs/AGENTS.md` and `README.md`.
- An agent can implement or modify the app from the spec alone.

## References

- `docs/app/spec.md` — the draft
- `docs/AGENTS.md` — docs table
- `README.md` — Accounting demo section

