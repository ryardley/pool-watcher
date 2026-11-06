---
id: pw-z7gq
status: open
deps: []
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [scanner]
---
# scanner: add sealedbox KeyService

## Description

The demo seals the viewing key client-side to the scanner's public key. Add a
`sealedbox` KeyService that generates an X25519 keypair at boot, exposes the
public key, and opens the sealed VK. This is the non-nitro path (the user may
run outside nitro). Keep `local` for tests; `nitro` stays a future note.

## Design

Envelope (shared with the browser bundle, `app/web/seal.ts`):

```
wire (base64):  ephemeralPublicKey(32) || nonce(24) || ciphertext(N+16)

seal(plaintext, recipientPub):
  ephPriv = x25519.utils.randomPrivateKey()
  ephPub  = x25519.getPublicKey(ephPriv)
  shared  = x25519.getSharedSecret(ephPriv, recipientPub)
  key     = hkdf(sha256, shared, salt="", info="railgun-scanner-vk", 32)
  ct      = xchacha20poly1305(key, nonce).encrypt(utf8(plaintext))

open(envelope, recipientPriv):
  shared = x25519.getSharedSecret(recipientPriv, ephPub)
  key    = hkdf(sha256, shared, salt="", info="railgun-scanner-vk", 32)
  -> xchacha20poly1305(key, nonce).decrypt(ct)
```

`plaintext` is the shareable viewing key hex string (UTF-8); after opening,
hex-decode to bytes (same as `local`).

1. `scanner/package.json` — add `@noble/curves`, `@noble/ciphers`,
   `@noble/hashes` (exact versions).
2. `scanner/src/key-service.ts`:
   - `SealedBoxKeyService implements KeyService`: boot-time
     `x25519.utils.randomPrivateKey()`; `publicKey()` returns base64;
     `decrypt()` opens the envelope and hex-decodes.
   - `createKeyService('sealedbox')` returns it.
3. `scanner/Dockerfile` — `ARG KEY_SERVICE=sealedbox` for the demo build.
4. Add a unit test: seal → open round-trip (and a wrong-key failure).
5. Update `docs/scanner/spec.md` (already drafted).

## Acceptance Criteria

- `KEY_SERVICE=sealedbox` scanner boots with a fresh X25519 keypair.
- `pubkey` returns the base64 public key.
- A VK sealed with the envelope opens to the original hex string; a different
  recipient key fails to open.
- `local` still works for the existing tests.

## References

- `docs/scanner/spec.md` — KeyService seam, sealedbox
- `docs/app/spec.md` — sealed-box envelope
- `scanner/src/key-service.ts` — existing `LocalKeyService` + `createKeyService`
- `app/web/seal.ts` — browser-side seal (must match the envelope exactly)

