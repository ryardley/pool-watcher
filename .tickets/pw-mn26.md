---
id: pw-mn26
status: open
deps: []
links: []
created: 2026-11-06T15:44:27Z
type: epic
priority: 1
tags: [accounting, demo]
---
# Accounting demo: actix-web UI + Beancount/Fava

Show that the scanner's transactions are tracked in a real open-source
accounting app. Local demo only, open source only, no PHP.

## Description

The repo already has a test bed (Anvil + RAILGUN V2 deployment + a scripted
shield / private-transfer / unshield scenario) and a stateless scanner that
trial-decrypts the proxy's logs for a viewing key. This epic wires them into a
demonstrable accounting pipeline:

1. `scripts/wallets.sh` generates wallets and prints addresses + viewing keys.
2. `docker compose up` starts the scanner, the app, and Fava.
3. Paste wallet B's viewing key into the app UI (`:8080`); it is sealed
   client-side to the scanner's X25519 public key and sent to the scanner,
   which becomes ready.
4. Fava (`:5000`) shows an empty ledger.
5. `scripts/scenario.sh` runs the shield / transfers / unshield scenario.
6. Refreshing Fava shows the transactions as merchant sales + a withdrawal.

Wallet B is the "merchant": it receives the 5 customer payments (product
memos) and later withdraws via unshield. The ledger is a merchant model.

## Design

`app/` is a Rust binary (actix-web) that:
- serves a UI where a viewing key is pasted and sealed client-side to the
  scanner's X25519 pubkey (X25519 sealed box, `@noble/curves` +
  `@noble/ciphers` + `@noble/hashes`);
- runs an alloy listener (backfill `eth_getLogs` then WS `eth_subscribe`)
  against the local Anvil chain, filtered by the proxy address;
- feeds the logs to the scanner over its Unix socket and collects events;
- writes a Beancount merchant ledger that Fava serves.

Anvil runs on the host (owned by the test bed's `chain.sh`), not in compose.
Compose runs `scanner-b` + `app` + `fava`. The app reaches Anvil via
`host.docker.internal:8545`.

Full spec: `docs/app/spec.md`.

## Sealed-box envelope

```
wire (base64):  ephemeralPublicKey(32) || nonce(24) || ciphertext(N+16)
key = hkdf(sha256, x25519(ephPriv, recipientPub), "", "railgun-scanner-vk", 32)
AEAD: xchacha20poly1305
```

## Ledger model (wallet B)

- `Assets:Railgun:B` — B's private balance
- `Income:Sales` — the 5 incoming customer payments
- `Assets:Bank:EOA2` — unshield destination
- `Expenses:Fees` — unshield fee
- Commodity `TST` (18 dp); change notes (`outputType == 2`) excluded

## Acceptance Criteria

- `scripts/wallets.sh` prints A/B addresses + shareable viewing keys.
- `docker compose up --build` starts `scanner-b`, `app`, `fava`.
- Pasting B's viewing key into `:8080` makes the scanner ready and starts the
  listener.
- Fava at `:5000` shows an empty ledger before the scenario.
- After `scripts/scenario.sh`, refreshing Fava shows 5 product-narrated sales
  + 1 withdrawal.
- `bean-check ledger.beancount` passes (double-entry balances).
- `Assets:Railgun:B` = 185 TST; `Income:Sales` = −235; `Assets:Bank:EOA2` =
  50 − fee; `Expenses:Fees` = fee.
- Scanner tests pass with the new `fee` field; `pubkey` + `sealedbox`
  round-trip works.

## References

- `docs/app/spec.md` — full app spec
- `docs/scanner/spec.md` — scanner protocol, KeyService, event shape
- `docs/testbed/spec.md` — test bed, `wallets.sh`, scenario memos
- `scanner/crates/handle/src/lib.rs` — Rust handle (wire-contract source of truth)

