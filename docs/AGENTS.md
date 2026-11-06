# AGENTS.md

Agent-facing entry point for this repository. Everything an agent needs to
orient and to keep working lives under `docs/`.

## What this repo is

A local RAILGUN V2 test bed (Anvil + fresh contract deployment + a scripted
shield / private-transfer / unshield scenario) plus a stateless scanner that
trial-decrypts the resulting history for a viewing key.

## How the docs are organized

| File | Purpose | How it changes |
|---|---|---|
| `docs/backlog.md` | The living board: **In progress / Next / Backlog / Done** | Grows as you work; items move down the columns |
| `docs/notes.md` | Findings, gotchas, workarounds | Append-only, short bullets, write as you hit them |
| `docs/testbed/spec.md` | Test bed reference spec (build, scripts, ops, scenario) | Reference; update when the test bed changes |
| `docs/scanner/spec.md` | Scanner reference spec (protocol, decryption, testing) | Reference; update when the scanner changes |

User-facing component docs stay with their components (`README.md`).

## Workflow

Dynamic and agile — no upfront full plan. Work a bit, then add the next thing
as you see it:

1. **Idea occurs** -> append a checkbox to **Backlog** in `docs/backlog.md`.
2. **About to start it** -> move the line up to **Next** (one or two items).
3. **Doing it** -> move to **In progress**.
4. **Finished** -> move to **Done** with a one-line outcome. If it surprised
   you, drop a bullet in `docs/notes.md`.

To add the next phase: write its items into the backlog, promote the first one
to **Next**, and start. The specs above are updated only when the component
they describe actually changes.

## Components

- **Test bed** — `docs/testbed/spec.md`. Local Anvil chain, RAILGUN V2
  deployment, `scripts/scenario.sh`, op scripts, keys/steps export.
- **Scanner** — `docs/scanner/spec.md`. Stateless trial-decryption service,
  one Unix socket per viewing key, Rust handle as the wire-contract source of
  truth.
