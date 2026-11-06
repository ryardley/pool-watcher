# App spec — actix-web + alloy listener + Beancount/Fava accounting demo

The integration app that turns the scanner's decrypted events into a real
accounting ledger. It is a Rust binary (`app/`) that:

- serves a small **actix-web** UI where a viewing key is pasted and sealed
  client-side to the scanner's X25519 public key;
- runs an **alloy** log listener (backfill + live subscribe) against the local
  Anvil chain;
- feeds the logs to the scanner over its Unix socket and collects events;
- writes a **Beancount** ledger that **Fava** serves as the accounting UI.

Local demo only. Open source only. No PHP.

## Goal

Show that the scanner's transactions are tracked in an accounting app:

1. `scripts/wallets.sh` generates wallets and prints addresses + viewing keys.
2. `docker compose up` starts the scanner, the app, and Fava.
3. Paste wallet B's viewing key into the app UI (`:8080`); it is sealed to the
   scanner's public key and sent to the scanner, which becomes ready.
4. Fava (`:5000`) shows an empty ledger.
5. `scripts/scenario.sh` runs the shield / transfers / unshield scenario.
6. Refreshing Fava shows the transactions as merchant sales + a withdrawal.

## Architecture

```
host: scripts/wallets.sh   -> Anvil + deployment + wallets + out/keys.json
host: scripts/scenario.sh  -> shield / 5 product-memo transfers / unshield

docker compose:
  scanner-b  (Node, KEY_SERVICE=sealedbox, Unix socket)
  app        (Rust: actix-web UI + alloy listener + scanner client + Beancount writer)
  fava       (Python, serves the ledger at :5000)

browser -> app :8080   (paste VK; JS seals to scanner pubkey)
app -> scanner : unix socket (pubkey, init, scan)
app -> Anvil   : host.docker.internal:8545 (backfill + WS subscribe)
app -> ledger  : shared volume -> fava :5000
```

Anvil runs on the host (owned by the test bed's `chain.sh`), not in compose.

## Locked decisions

- Accounting app: **Beancount + Fava** (file-based, no PHP).
- Scanner: **one instance, wallet B only** (the merchant).
- Listener: **alloy**, backfill (`eth_getLogs`) then WS subscribe.
- Ledger: **merchant model** — `Assets:Railgun:B`, `Income:Sales`,
  `Assets:Bank:EOA2`, `Expenses:Fees`, commodity `TST` (18 dp); change notes
  (`outputType == 2`) excluded.
- Unshield fee: the scanner's unshield event exposes `fee` so the writer can
  post net to the bank and fee to expenses.
- Idempotency: **in-memory accumulate + atomic full rewrite** of the ledger.
- VK encryption: **X25519 sealed box** (`@noble/curves` + `@noble/ciphers` +
  `@noble/hashes`), same code in browser and scanner.
- Compose: **Anvil on host**; `scanner-b` + `app` + `fava`.
- Crate: **`./app`**; compose: **repo-root `docker-compose.yml`**.

## Repo layout

```
app/
  Cargo.toml        # actix-web, actix-files, tokio, alloy, serde, anyhow,
                    # base64, chrono, bigdecimal, railgun-scanner-handle (path)
  Dockerfile        # multi-stage: esbuild static bundle + Rust build
                    # (build context = repo root, to include scanner/crates/handle)
  web/              # seal.ts + package.json (noble deps, esbuild)
  static/           # index.html + built seal.js
  src/
    main.rs         # actix server + spawn the listener
    config.rs       # read .state/deployment.json (proxy, deploymentBlock)
    scanner.rs      # dedicated thread + mpsc; pubkey/init/scan via the handle
    listener.rs     # alloy HTTP backfill + WS subscribe, filter by proxy, group by block
    ledger.rs       # event -> Beancount, dedup set, atomic rewrite
    web.rs          # GET /, GET /api/scanner-pubkey, POST /api/init, GET /api/status
```

## Sealed-box envelope

The viewing key never crosses the socket boundary in plaintext. The browser
seals it to the scanner's public key; the scanner opens it with its private
key. The app only relays ciphertext.

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

- `plaintext` is the shareable viewing key hex string (UTF-8).
- The scanner generates its X25519 keypair at boot; the public key is served
  via the scanner's `pubkey` op and proxied by the app.
- Browser bundle: `app/web/seal.ts` built with esbuild into `app/static/seal.js`.

## Web UI / API (actix-web, `:8080`)

| Route | Purpose |
|---|---|
| `GET /` | Serves `index.html` + `seal.js`. Form: paste the viewing key. |
| `GET /api/scanner-pubkey` | Proxies the scanner `pubkey` op; returns `{ publicKey }` (base64). |
| `POST /api/init` | Body `{ keyCiphertext }` (base64 sealed VK); forwards to the scanner `init`; returns `{ walletAddress }`; starts the listener. |
| `GET /api/status` | Scanner state: `{ ready, walletAddress }`. |

Flow: the page fetches the pubkey, seals the pasted VK with `seal.js`, POSTs
the ciphertext to `/api/init`. The app forwards it to the scanner over the
socket; the scanner opens it, derives the wallet, and enters ready state.

## Scanner client

- Dedicated thread + mpsc channel: the sync `railgun-scanner-handle` is called
  from async actix/alloy code via the channel.
- Ops: `pubkey()`, `init(keyCiphertext)`, `scan(blocks)`, `ping()`.
- Waits for the socket to exist before connecting.

## Alloy listener

- Reads `proxy` and `deploymentBlock` from `.state/deployment.json`.
- **Backfill**: `eth_getLogs` from `deploymentBlock` to latest, filtered by the
  proxy address.
- **Subscribe**: WS `eth_subscribe` to new logs, filtered by the proxy address.
- Groups logs by block into `{ chainId, logs }` batches and forwards them to
  the scanner (`scan`).
- Fetches each block's timestamp (`eth_getBlockByNumber`) for the Beancount
  date.
- Starts after the scanner is initialized; the scanner ignores non-Railgun
  logs, so filtering by proxy address alone is sufficient.

## Beancount ledger writer

- Keeps all events in memory, deduped by `txHash:logIndex:hash:direction`
  (unshield: `txHash:logIndex:unshield`).
- Rewrites `ledger.beancount` atomically (temp file + rename) on every change;
  Fava reloads on the rename. Restart = rebuild from backfill, so re-runs are
  idempotent.
- Header: `option "operating_currency" "TST"` + `open` directives for the four
  accounts.
- Amounts: base units ÷ 10¹⁸, formatted to 18 dp (bigdecimal or manual string
  insert).

## Event → Beancount mapping (wallet B)

| Scanner event | Beancount entry |
|---|---|
| `transact`, `direction=in`, `outputType=0` (received note) | `Assets:Railgun:B +value` / `Income:Sales -value`; narration = product memo; metadata `txhash`, `block`, `logindex`, `wallet`, `token` |
| `unshield` | `Assets:Railgun:B -value` / `Assets:Bank:EOA2 +(value-fee)` / `Expenses:Fees +fee`; narration `Withdrawal` |
| `transact`, `outputType=2` (change note) | excluded |
| `transact`, `direction=out` (sent note) | excluded (B only sends via unshield) |

`value` is the gross amount leaving the private balance; `fee` is the unshield
fee. `value - fee` is the net the EOA receives.

## Config / env

| Env | Default | Meaning |
|---|---|---|
| `RPC_URL` | `http://host.docker.internal:8545` | Anvil HTTP |
| `WS_URL` | `ws://host.docker.internal:8545` | Anvil WS |
| `CHAIN_ID` | `31337` | Chain id echoed into scan batches |
| `SCANNER_SOCKET` | `/run/scanner/scanner-b.sock` | Scanner Unix socket |
| `DEPLOYMENT_PATH` | `/data/.state/deployment.json` | Proxy + deployment block |
| `LEDGER_PATH` | `/ledger/ledger.beancount` | Beancount output |
| `BIND_ADDR` | `0.0.0.0:8080` | actix-web bind |

## Docker / compose

`docker-compose.yml` (repo root):

- **`scanner-b`** — `scanner/Dockerfile`, `KEY_SERVICE=sealedbox`,
  `SCANNER_SOCKET=/run/scanner/scanner-b.sock`, volume `scanner-sock:/run/scanner`.
- **`app`** — `app/Dockerfile`, env above, `extra_hosts:
  ["host.docker.internal:host-gateway"]`, volumes `scanner-sock`,
  `./.state:/data/.state:ro`, `ledger:/ledger`, ports `8080:8080`,
  `depends_on: scanner-b`.
- **`fava`** — `python:3-slim` + `pip install fava`; entrypoint waits for
  `/ledger/ledger.beancount` to exist, then `fava /ledger/ledger.beancount
  --host 0.0.0.0 --port 5000`; volume `ledger:/ledger`, ports `5000:5000`,
  `depends_on: app`.

## Demo flow

1. `scripts/wallets.sh` — prints A/B addresses + shareable viewing keys.
2. `docker compose up --build` (separate terminal).
3. Open `http://localhost:8080` (actix-web UI).
4. Paste B's viewing key — the browser seals it to the scanner's pubkey, the
   app forwards it, the scanner becomes ready, the listener starts.
5. Open `http://localhost:5000` (Fava) — empty ledger.
6. `scripts/scenario.sh`.
7. Refresh Fava — 5 product-narrated sales + 1 withdrawal.

## Verification

- `bean-check ledger.beancount` passes (double-entry balances).
- Fava shows 5 sales + 1 withdrawal.
- `Assets:Railgun:B` = 235 − 50 = 185 TST; `Income:Sales` = −235;
  `Assets:Bank:EOA2` = 50 − fee; `Expenses:Fees` = fee.
- Scanner tests pass with the `fee` field; `pubkey` + `sealedbox` round-trip
  works.

## Testing

- **Sealed box**: unit round-trip (seal → open) in the scanner and the browser
  bundle.
- **Ledger writer**: feed fixture events → assert the Beancount text and
  balances.
- **End-to-end**: the demo flow above, plus `bean-check`.
