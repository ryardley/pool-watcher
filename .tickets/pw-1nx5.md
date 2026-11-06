---
id: pw-1nx5
status: closed
deps: [pw-z7gq]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [scanner]
---
# scanner: add pubkey op

## Description

The browser must seal the viewing key to the scanner's X25519 public key
before sending it. Add a `pubkey` op so the app can fetch that public key and
serve it to the browser. Only meaningful when `KEY_SERVICE=sealedbox`; the
`local` service returns a placeholder or errors.

## Design

Contract-first: the Rust handle defines the wire structs, the Node scanner
mirrors them.

1. `scanner/crates/handle/src/lib.rs`:
   - Add `Pubkey` request struct (`{ op: "pubkey" }`).
   - Add a `PubkeyResponse`/`Response::Pubkey { public_key }` variant.
   - Add `pub fn pubkey(&mut self) -> Result<String, HandleError>` returning
     the base64 public key.
2. `scanner/src/wire.ts`:
   - Add `{ op: z.literal('pubkey') }` to `RequestSchema`.
   - Add `PubkeyResponseSchema` (`{ op: "pubkey", publicKey: string }`) to
     `ResponseSchema`.
3. `scanner/src/server.ts` — handle the `pubkey` request by calling
   `keyService.publicKey()`; return `{ op: "pubkey", publicKey }`.
4. `scanner/src/key-service.ts` — add `publicKey(): string` to the
   `KeyService` interface; `LocalKeyService` returns a placeholder (e.g. empty
   string) or throws.
5. Update `docs/scanner/spec.md` (already drafted).

## Acceptance Criteria

- `{ "op": "pubkey" }` returns `{ "op": "pubkey", "publicKey": "<base64>" }`.
- The Rust handle exposes `pubkey()` and parses the response.
- With `KEY_SERVICE=local`, `pubkey` returns a placeholder or a clear error
  (never crashes).
- Protocol tests cover `pubkey` (success + error path).

## References

- `docs/scanner/spec.md` — `pubkey` op
- `scanner/crates/handle/src/lib.rs` — wire structs + handle API
- `scanner/src/server.ts` — request dispatch
- `scanner/src/key-service.ts` — `KeyService` interface

