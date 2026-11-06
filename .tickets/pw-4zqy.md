---
id: pw-4zqy
status: closed
deps: []
links: []
created: 2026-11-06T15:45:27Z
type: task
priority: 2
tags: [testbed]
---
# Test bed: local Anvil chain + RAILGUN V2 deployment + scenario

## Description

The test bed: a local Anvil chain, a fresh RAILGUN V2 deployment, and a
scripted scenario (shield, 5x A→B transfers, unshield) with steps/keys
export. See `docs/testbed/spec.md`.

## Acceptance Criteria

- `scripts/scenario.sh` runs end to end on a fresh clone.
- `out/steps.jsonl`, `out/keys.json`, and `.state/deployment.json` exist and
  are consistent.
- `scripts/setup.sh` a second time prints only `skip:` lines.
- `scripts/reset.sh && scripts/scenario.sh` works without re-downloading
  circuits or reinstalling packages.

## References

- `docs/testbed/spec.md`

