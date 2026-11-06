---
id: pw-sg82
status: open
deps: [pw-fxzs]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 2
parent: pw-mn26
tags: [docs]
---
# docs: update AGENTS.md + specs

## Description

Keep the docs consistent with the accounting demo: the docs table, the
component list, and the scanner/testbed specs must reflect the new protocol
(`fee`, `pubkey`, `sealedbox`), the new scripts (`wallets.sh`), and the
product memos. Most of this is already drafted as context; verify and finish.

## Design

- `docs/AGENTS.md` — docs table + Components (add `docs/app/spec.md`, App
  component; tk workflow already updated).
- `docs/scanner/spec.md` — event `fee`, `pubkey` op, `sealedbox` KeyService
  (already drafted; verify).
- `docs/testbed/spec.md` — `wallets.sh`, product memos, repo layout (already
  drafted; verify).
- `README.md` — `wallets.sh` command + Accounting demo section (already
  drafted; verify).

## Acceptance Criteria

- No stale references to `docs/backlog.md` anywhere.
- `docs/AGENTS.md` lists `docs/app/spec.md` and the App component.
- `docs/scanner/spec.md` documents `fee`, `pubkey`, and `sealedbox`.
- `docs/testbed/spec.md` documents `wallets.sh` and the product memos.
- `README.md` mentions `wallets.sh` and the accounting demo.

## References

- `docs/AGENTS.md`, `docs/scanner/spec.md`, `docs/testbed/spec.md`,
  `README.md`

