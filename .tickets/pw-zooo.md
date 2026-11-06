---
id: pw-zooo
status: closed
deps: [pw-szlj, pw-jwrq]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 2
parent: pw-mn26
tags: [docker]
---
# docker: app/Dockerfile

## Description

Containerize the Rust app. Multi-stage: build the browser seal bundle with
esbuild, build the Rust binary, and ship a slim runtime image.

## Design

- **Build context = repo root** (the crate path-depends on
  `scanner/crates/handle`, so the context must include both `app/` and
  `scanner/crates/handle/`).
- Stage 1 (node): `npm ci` in `app/web/`, `esbuild app/web/seal.ts --bundle
  --minify --outfile=app/static/seal.js`.
- Stage 2 (rust): `cargo build --release` for the `app` package.
- Stage 3 (runtime): slim base (e.g. `debian:bookworm-slim` or
  `gcr.io/distroless/cc`), copy the binary + `app/static/`.
- The image needs no network at runtime (Anvil is reached via
  `host.docker.internal`).

## Acceptance Criteria

- `docker build -f app/Dockerfile .` (context = repo root) succeeds.
- The image runs the app binary; `app/static/seal.js` is present.
- The image is small enough for a local demo (no dev toolchain).

## References

- `docs/app/spec.md` — Repo layout, Docker/compose
- `scanner/Dockerfile` — existing multi-stage pattern to mirror
- `app/Cargo.toml` — the path dependency that forces the repo-root context

