---
id: pw-szlj
status: closed
deps: []
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: scaffold app/ crate

## Description

Create the Rust binary crate at repo root `./app` that hosts the actix-web
UI, the alloy listener, the scanner client, and the Beancount writer. This
ticket is the skeleton: Cargo.toml, module files, and a runnable main that
parses config.

## Design

```
app/
  Cargo.toml        # package + deps
  src/
    main.rs         # actix server + spawn the listener
    config.rs       # read .state/deployment.json (proxy, deploymentBlock)
    scanner.rs      # dedicated thread + mpsc; pubkey/init/scan via the handle
    listener.rs     # alloy HTTP backfill + WS subscribe
    ledger.rs       # event -> Beancount, dedup set, atomic rewrite
    web.rs          # GET /, GET /api/scanner-pubkey, POST /api/init, GET /api/status
```

`Cargo.toml` dependencies (pin current versions):

- `actix-web`, `actix-files` — HTTP server + static assets
- `tokio` (rt-multi-thread, macros, sync) — async runtime
- `alloy` (provider-http, provider-ws, rpc-types) — Anvil client
- `serde`, `serde_json` — config + wire
- `anyhow` — error handling
- `base64` — pubkey / ciphertext encoding
- `chrono` — block timestamp -> Beancount date
- `bigdecimal` — base units / 1e18 formatting (or manual string insert)
- `railgun-scanner-handle = { path = "../scanner/crates/handle" }`

Note: the crate path-depends on `scanner/crates/handle`, so the Docker build
context must be the repo root (see `docker: app/Dockerfile`).

`config.rs` reads `.state/deployment.json` (mounted read-only) for `proxy` and
`deploymentBlock`, plus env vars (see `docs/app/spec.md` Config table).

## Acceptance Criteria

- `cargo build` succeeds in `app/` (with the scanner workspace available).
- `main.rs` starts an actix-web server on `BIND_ADDR` and serves a placeholder
  page.
- `config.rs` parses `.state/deployment.json` and the env vars, with clear
  errors when files are missing.

## References

- `docs/app/spec.md` — Repo layout, Config/env
- `scanner/crates/handle/Cargo.toml` — the path dependency target
- `scanner/Cargo.toml` — existing workspace (do not disturb)

