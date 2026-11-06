---
id: pw-jwrq
status: open
deps: [pw-z7gq, pw-ctx4]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [app]
---
# app: browser seal bundle + static assets

## Description

The browser must seal the pasted viewing key to the scanner's X25519 public
key before sending it. Build a small JS bundle from `app/web/seal.ts` (noble
deps) into `app/static/seal.js` and serve it with `index.html`.

## Design

- `app/web/seal.ts` — implements the sealed-box envelope (must match the
  scanner's `sealedbox` KeyService exactly):
  ```
  wire (base64):  ephemeralPublicKey(32) || nonce(24) || ciphertext(N+16)
  key = hkdf(sha256, x25519(ephPriv, recipientPub), "", "railgun-scanner-vk", 32)
  AEAD: xchacha20poly1305
  ```
- `app/web/package.json` — deps `@noble/curves`, `@noble/ciphers`,
  `@noble/hashes`, devDep `esbuild`.
- Build: `esbuild app/web/seal.ts --bundle --minify --outfile=app/static/seal.js`.
- `app/static/index.html` — the form page; JS fetches `/api/scanner-pubkey`,
  seals the VK, POSTs to `/api/init`.
- The bundle is built in the Dockerfile (see `docker: app/Dockerfile`).

## Acceptance Criteria

- `seal.js` builds with esbuild and loads in a browser.
- A VK sealed by the bundle opens with the scanner's `sealedbox` private key
  (round-trip test).
- The page flow works: fetch pubkey → seal → POST → show wallet address.

## References

- `docs/app/spec.md` — Sealed-box envelope
- `scanner/src/key-service.ts` — the matching `sealedbox` open side
- `docs/scanner/spec.md` — KeyService seam

