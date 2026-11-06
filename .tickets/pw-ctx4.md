---
id: pw-ctx4
status: closed
deps: [pw-ixzj]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: actix-web UI + API

## Description

The user-facing entry point: a small web UI where the viewing key is pasted,
sealed client-side to the scanner's X25519 pubkey, and sent to the scanner.
The app relays ciphertext only — it never sees the plaintext VK.

## Design

Routes:

| Route | Purpose |
|---|---|
| `GET /` | Serves `index.html` + `seal.js`. Form: paste the viewing key. |
| `GET /api/scanner-pubkey` | Proxies the scanner `pubkey` op; returns `{ publicKey }` (base64). |
| `POST /api/init` | Body `{ keyCiphertext }` (base64 sealed VK); forwards to the scanner `init`; returns `{ walletAddress }`; starts the listener. |
| `GET /api/status` | Scanner state: `{ ready, walletAddress }`. |

Flow: the page fetches the pubkey, seals the pasted VK with `seal.js`, POSTs
the ciphertext to `/api/init`. The app forwards it to the scanner over the
socket; the scanner opens it, derives the wallet, and enters ready state.

- Serve static assets with `actix-files` (or `include_str!`).
- The scanner client thread is shared via actix app state (`web::Data`).
- Handle the "already initialized" error from a second `init` gracefully.

## Acceptance Criteria

- `GET /` serves the UI; `seal.js` loads.
- `GET /api/scanner-pubkey` returns the scanner's base64 public key.
- `POST /api/init` with a sealed VK returns `{ walletAddress }` and starts the
  listener; a second init returns a clear error.
- `GET /api/status` reflects ready state.

## References

- `docs/app/spec.md` — Web UI / API section
- `scanner/src/server.ts` — the `init`/`pubkey` handlers the app proxies

