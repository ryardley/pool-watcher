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
| `.tickets/` | tk tickets: the living board (open / in_progress / closed, deps, priorities) | `tk create/start/close/dep` |
| `docs/notes.md` | Findings, gotchas, workarounds | Append-only, short bullets, write as you hit them |
| `docs/testbed/spec.md` | Test bed reference spec (build, scripts, ops, scenario) | Reference; update when the test bed changes |
| `docs/scanner/spec.md` | Scanner reference spec (protocol, decryption, KeyService, testing) | Reference; update when the scanner changes |
| `docs/app/spec.md` | Accounting demo spec: actix-web UI + alloy listener + Beancount/Fava | Reference; update when the integration app changes |

User-facing component docs stay with their components (`README.md`).

## Task tracking (tk)

This project uses a CLI ticket system for task management. Run `tk help` when
you need to use it. Tickets are markdown files with YAML frontmatter in
`.tickets/` (git-native, no daemon, no DB).

Workflow:

1. **Idea occurs** -> `tk create "<title>" -d "<context>"` (optionally
   `--parent <epic>` to attach to a phase, `--tags`, `-p` priority).
2. **About to start it** -> it appears in `tk ready` once its dependencies are
   resolved.
3. **Doing it** -> `tk start <id>`.
4. **Finished** -> `tk close <id>`; if it surprised you, drop a bullet in
   `docs/notes.md`.

Useful commands: `tk ready` (deps resolved), `tk blocked` (waiting on deps),
`tk show <id>`, `tk dep <id> <dep-id>`, `tk dep tree <id>`, `tk closed`.

To add the next phase: create an epic (`-t epic`), create its tickets with
`--parent <epic>`, and wire up `tk dep` edges. The specs above are updated only
when the component they describe actually changes.

## Components

- **Test bed** — `docs/testbed/spec.md`. Local Anvil chain, RAILGUN V2
  deployment, `scripts/scenario.sh`, op scripts, keys/steps export.
- **Scanner** — `docs/scanner/spec.md`. Stateless trial-decryption service,
  one Unix socket per viewing key, Rust handle as the wire-contract source of
  truth.
- **App** — `docs/app/spec.md`. Accounting demo: actix-web UI (paste a viewing
  key, sealed client-side to the scanner's X25519 pubkey), alloy listener,
  Beancount merchant ledger served by Fava.
