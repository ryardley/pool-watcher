# scanner

Stateless Railgun trial-decryption service. One process + one Unix socket per
viewing key. It trial-decrypts `Shield` / `Transact` / `Unshield` logs for a
viewing key and returns the events it found. It never spends, so it holds no
merkle tree, no database, no RPC and no network access.

See `../docs/scanner/spec.md` for the full specification.

## Build and run

```sh
cd scanner
npm ci
npm run build
SCANNER_SOCKET=/run/scanner.sock node dist/index.js
```

`SCANNER_SOCKET` is the Unix socket path. `KEY_SERVICE` selects the KeyService
implementation (`local` by default); it is a build/deploy property, never a wire
input.

## Wire protocol

NDJSON over a Unix domain socket: one JSON object per line, exactly one response
line per request. The Rust handle in `crates/handle` is the source of truth for
the wire structs; the Node scanner mirrors them.

```
init  { keyCiphertext }                          -> ready { walletAddress }
scan  { blocks: [{ chainId, logs: [raw logs] }] } -> events { events: [...] }
pubkey                                            -> pubkey { publicKey }
ping                                              -> pong
```

Failures respond `{ "op": "error", "message": "..." }`. The scanner is
single-flight: one client connection at a time, one scan at a time.

## KeyService

The viewing key never crosses the socket boundary and never touches disk. The
core depends on one interface:

```ts
interface KeyService {
  decrypt(keyCiphertext: string): Promise<Uint8Array>;
  publicKey(): string;
}
```

`local` (dev): `keyCiphertext` is `base64(viewingKeyHex)`, where `viewingKeyHex`
is the shareable viewing key; `publicKey()` throws (no real public key).
`sealedbox`
(demo): the scanner generates an X25519 keypair at boot, `publicKey()` returns
the base64 public key, and `decrypt` opens a sealed box (X25519 + HKDF-SHA256 +
XChaCha20-Poly1305) to recover the viewing key. `nitro` is a future build-time
implementation.

## Tests

```sh
# Unit (Node, node:test + tsx, in-process)
npm test

# Protocol + acceptance (Rust handle; spawns the real scanner)
cargo test

# End-to-end acceptance: restart chain, eth_getLogs for real, scan, assert
scripts/acceptance.sh
```

The unit and protocol tests consume `../out/logs.json`, `../out/keys.json` and
`../out/steps.jsonl`, produced by the test bed (`scripts/scenario.sh`).

## Docker

```sh
docker build --build-arg KEY_SERVICE=local -t railgun-scanner .
docker run -v /run/scanner:/run railgun-scanner
```

The image entrypoint is `node dist/index.js`.
