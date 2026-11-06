# Scanner spec — stateless Railgun trial-decryption service

A component for an accounting system that tracks wallets for transactions. It
trial-decrypts Railgun events for a viewing key and returns the events it
found. It can never spend, so it holds no merkle tree, no database, no state.

## Shape

```
init  { keyCiphertext }                          -> ready
scan  { blocks: [{ chainId, logs: [raw logs] }] } -> events: [...]
pubkey                                              -> pubkey
ping                                                -> pong
```

- One process + one Unix socket per viewing key.
- The scanner is key-bound, chain-agnostic and asset-agnostic: a single
  instance serves all chains, all assets, all pools (pool == chain).
- Pure decrypt primitive: no merkle tree, no DB, no RPC, no network access.
- Inputs are pre-filtered `Shield` / `Transact` / `Unshield` logs from the
  Railgun proxy, passed as raw RPC log objects per block. The scanner parses
  them with the contract ABI; it needs no proxy-address config.
- Output is one event per decrypted note/commitment. `Unshield` is public and
  passes through unattributed.

## Protocol

- Unix domain socket, one per instance, path from `SCANNER_SOCKET`
  (e.g. `/run/scanner.sock`), mounted into the container.
- NDJSON framing: one JSON object per line; request -> exactly one response
  line. No HTTP.
- Single-flight: one client, one scan at a time.
- On failure respond `{ "op": "error", "message": "..." }`.

### init

```
{ "op": "init", "keyCiphertext": "<base64>" }
```

- Sent once at startup; a second init is an error.
- `KeyService.decrypt(keyCiphertext)` -> viewing key bytes.
- Derive the wallet's `currentWalletAddressData` and 0zk address from the key.
- Enter ready state. The key lives in memory only; never written to disk.

### scan

```
{ "op": "scan", "blocks": [
    { "chainId": 31337,
      "logs": [ <raw log: address, topics, data, blockNumber, logIndex, transactionHash> ] }
  ] }
```

- Caller batches freely and may mix chains in one request.
- Returns `{ "op": "events", "events": [...] }`.

### pubkey

```
{ "op": "pubkey" } -> { "op": "pubkey", "publicKey": "<base64>" }
```

- Returns the scanner's X25519 public key so a client can seal a viewing key
  to it. Only meaningful when `KEY_SERVICE=sealedbox`; the `local` service
  returns a placeholder or errors.

### event

```
{ "chainId", "walletAddress", "address", "blockNumber", "txHash", "logIndex",
  "kind": "shield" | "transact" | "unshield",
  "hash", "token", "value", "fee", "outputType",
  "direction": "in" | "out",
  "sender", "receiver", "memo" }
```

- `walletAddress` is the 0zk address derived from the viewing key at init, so
  events are self-describing.
- `chainId` (the pool link) and log linkage (`address`, `blockNumber`,
  `txHash`, `logIndex`) are echoed into every event.
- `direction` is derived per-note from the trial-decrypt outcome, no state.
- `fee` is non-null only for `unshield`: the contract emits
  `Unshield(to, token, base, fee)` where `base` is already net of `fee`; the
  scanner reports `value = base + fee` (the gross leaving the private balance)
  and `fee` separately, so consumers can post `net = value - fee`.

## Decryption

- Node/TypeScript. Import only decryption bits from
  `@railgun-community/engine` (note classes, not the wallet/engine runtime):
  - `ShieldNote.decryptRandom` for shield bundles;
  - `TransactNote.decrypt` for transact bundles;
  - `getTokenDataERC20` for the `tokenDataGetter` shim (ERC-20 tokenHash is
    the token address zero-padded; ~5 lines, no DB, no chain call).
- Trial decrypt: for each `Transact` ciphertext, derive a shared key from the
  viewing key and the blinded receiver key (try `isSentNote=false`), then the
  blinded sender key (`isSentNote=true`); a passing AES-GCM auth means the
  note belongs to this wallet. Shields use `shieldKey` similarly.
- `chain` arg to `TransactNote.decrypt` is `{ type: 0, id: chainId }` only.

## KeyService seam

The viewing key never crosses the socket boundary and never touches disk.
The scanner core depends on a single interface:

```ts
interface KeyService {
  decrypt(keyCiphertext: string): Promise<Uint8Array>;  // viewing key bytes
  publicKey(): string;                                   // X25519 public key, base64
}
```

- **local** (dev, tests): `keyCiphertext` is `base64(viewingKeyHex)`. No real
  crypto, same wire shape. `publicKey()` returns a placeholder.
- **sealedbox** (demo): the scanner generates an X25519 keypair at boot;
  `publicKey()` returns the base64 public key; `decrypt` opens a sealed box
  (X25519 + HKDF-SHA256 + XChaCha20-Poly1305 via
  `@noble/curves`/`@noble/ciphers`/`@noble/hashes`) to recover the viewing-key
  hex string, then hex-decodes it. Envelope: `ephemeralPublic(32) || nonce(24)
  || ciphertext`; key = `hkdf(sha256, shared, "", "railgun-scanner-vk", 32)`.
- **nitro** (future, not built now): customer ECIES-encrypts the viewing key
  to a private key generated at enclave boot (pubkey distributed via
  attestation); `decrypt` uses the in-enclave key. No outbound network.
- Selected at image build time (build arg), not at runtime, not on the wire.
  `nitro-cli run-enclave` cannot pass env vars, so this is a build/deploy
  property.

## Layout

```
scanner/
  src/            # Node scanner (socket server)
  crates/handle/  # Rust client: wire structs + connect/send/recv
  tests/          # protocol tests: spawn the scanner, drive it via the Rust handle
```

The task specification lives in `../docs/scanner/spec.md` (this file), not in
the component directory.

## Rust handle (production client)

The accounting system is Rust. A Rust handle library talks to the scanner over
the socket. Its request/response structs are the **source of truth for the
wire contract**; the Node scanner mirrors them (contract-first).

- Thin and sync: std `UnixStream` + `serde`/`serde_json`. No tokio.
- Defines the wire structs: `Init`, `Scan`, `Pubkey`, `Ping`, `Ready`,
  `Events`, `Pong`, `Error`, `Event` — one JSON object per line, matching the
  protocol above. `Event` carries `fee: Option<String>`.
- Exposes a small API: `connect(path)`, `init(keyCiphertext)`,
  `scan(blocks)`, `pubkey()`, `ping()`.
- Built and tested alongside the scanner; also used as the level-3 protocol
  test client.
- Requires the Rust toolchain: add `languages.rust.enable = true` to
  `devenv.nix`.

## Testing

Layered, cheapest first. The test bed generates ground-truth history.

1. **Fixtures.** The test bed also dumps the proxy's
   `Shield`/`Transact`/`Unshield` logs (deployment block -> head) to a JSON
   fixture, alongside `out/steps.jsonl` and `out/keys.json`. Tests consume the
   fixture directly — no chain, no RPC, no Docker.
2. **Unit (Node, `node:test` + `tsx`, in-process).** `KeyService` local impl,
   message parse/validate, `tokenDataGetter` shim, and the decrypt logic
   against the fixture with A's key and B's key — assert events reconstruct
   `out/steps.jsonl` and the balances arithmetic. Also event shaping: linkage
   echo, `direction`, memo.
3. **Protocol (spawn the scanner, drive it via the Rust handle).** `init` ->
   `ready`; `scan` (fixture) -> `events`; `ping` -> `pong`; error cases (init
   twice, scan before init, malformed JSON, unknown op); single-flight.
   Cross-language, so it tests the real wire contract, not Node agreeing with
   Node.
4. **End-to-end acceptance.** Restart the chain, `eth_getLogs` for real, scan,
   assert — test bed -> logs -> scanner -> events are consistent.

## Packaging

- Docker base `node:26-alpine` (fallback `node:24-alpine` if the 2025-era SDK
  misbehaves; no `engines` constraint, deps are pure JS/wasm).
- `ARG KEY_SERVICE=local` wires the KeyService implementation at build time;
  the accounting demo builds with `KEY_SERVICE=sealedbox`.
- Entrypoint: single `node dist/index.js` that binds the socket and serves
  the protocol.

## Acceptance

Use the test bed to generate ground-truth history, then:

1. Restart the chain (`scripts/chain.sh`) so the full history is queryable.
2. `eth_getLogs` the proxy's `Shield`/`Transact`/`Unshield` logs from the
   deployment block to head.
3. Feed them to the scanner as `scan` requests, batched by block, with wallet
   A's key and separately B's (driven via the Rust handle).
4. Assert the decrypted events reconstruct `out/steps.jsonl`: shield of 1000
   to A, the five A->B transfers (10/25/40/60/100, memos, sender visibility),
   and B's unshield of 50 — consistent with the balances arithmetic.
