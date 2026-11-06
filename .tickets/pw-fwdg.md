---
id: pw-fwdg
status: open
deps: [pw-zooo]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 2
parent: pw-mn26
tags: [docker]
---
# docker: repo-root docker-compose.yml

## Description

The demo's one-command stack: `docker compose up --build` starts the scanner,
the app, and Fava. Anvil stays on the host (owned by the test bed).

## Design

Services:

- **`scanner-b`** — `scanner/Dockerfile`, `KEY_SERVICE=sealedbox`,
  `SCANNER_SOCKET=/run/scanner/scanner-b.sock`, volume `scanner-sock:/run/scanner`.
- **`app`** — `app/Dockerfile`, env:
  - `RPC_URL=http://host.docker.internal:8545`
  - `WS_URL=ws://host.docker.internal:8545`
  - `CHAIN_ID=31337`
  - `SCANNER_SOCKET=/run/scanner/scanner-b.sock`
  - `DEPLOYMENT_PATH=/data/.state/deployment.json`
  - `LEDGER_PATH=/ledger/ledger.beancount`
  - `BIND_ADDR=0.0.0.0:8080`
  - `extra_hosts: ["host.docker.internal:host-gateway"]`
  - volumes: `scanner-sock`, `./.state:/data/.state:ro`, `ledger:/ledger`
  - ports `8080:8080`, `depends_on: scanner-b`
- **`fava`** — `python:3-slim` + `pip install fava`; entrypoint waits for
  `/ledger/ledger.beancount` to exist, then `fava /ledger/ledger.beancount
  --host 0.0.0.0 --port 5000`; volume `ledger:/ledger`, ports `5000:5000`,
  `depends_on: app`.

Named volumes: `scanner-sock`, `ledger`.

## Acceptance Criteria

- `docker compose up --build` starts all three services.
- The app reaches Anvil on the host via `host.docker.internal:8545`.
- Fava serves `:5000` and waits for the ledger file.
- `docker compose down` cleans up cleanly.

## References

- `docs/app/spec.md` — Docker/compose section, Config/env table
- `scanner/Dockerfile` — scanner image
- `app/Dockerfile` — app image

