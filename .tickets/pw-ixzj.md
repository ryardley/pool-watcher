---
id: pw-ixzj
status: open
deps: [pw-szlj]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: scanner client thread

## Description

The `railgun-scanner-handle` is a sync std `UnixStream` client, but the app is
async (actix-web + tokio/alloy). Bridge the two with a dedicated thread + mpsc
channel so async code can call `pubkey`, `init`, `scan`, and `ping` without
blocking the runtime.

## Design

- Spawn a thread that owns the `Handle` (connect to `SCANNER_SOCKET`).
- An mpsc channel carries requests; the thread calls the sync handle and sends
  responses back.
- Ops exposed to the rest of the app:
  - `pubkey() -> String` (base64 X25519 public key)
  - `init(keyCiphertext) -> String` (wallet address)
  - `scan(blocks) -> Vec<Event>`
  - `ping()`
- Wait for the socket file to exist before connecting (retry loop with
  timeout); the scanner container may start after the app.
- The scanner is single-flight: serialize scan calls through the channel.

## Acceptance Criteria

- The app can `pubkey()`, `init()`, and `scan()` against a running scanner
  without blocking the actix runtime.
- If the socket is missing, the client retries and eventually connects (or
  fails with a clear error after a timeout).
- `scan` requests are serialized (single-flight).

## References

- `scanner/crates/handle/src/lib.rs` — the sync handle API
- `docs/app/spec.md` — Scanner client section

