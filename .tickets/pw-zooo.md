---
id: pw-zooo
status: open
deps: [pw-szlj, pw-jwrq]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 2
parent: pw-mn26
tags: [docker]
---
# docker: app/Dockerfile

Multi-stage: esbuild static bundle + Rust build. Build context = repo root so the crate can path-depend on scanner/crates/handle.

