# Backlog

The living, dynamic board for this repo. Work a bit, then add the next thing
as you think of it. Keep items short and actionable.

**Workflow**

1. Idea occurs -> append a checkbox to **Backlog**.
2. About to start it -> move the line up to **Next** (one or two items).
3. Doing it -> move to **In progress**.
4. Finished -> move to **Done** and append a one-line outcome. If it surprised
   you, also drop a bullet in `docs/notes.md`.

No upfront full plan needed; the board grows as you work.

## In progress

- [ ]

## Next

- [ ] Accounting demo: scanner `fee` + `pubkey` + `sealedbox` KeyService

## Backlog

Accounting demo (actix-web UI + Beancount/Fava). Full spec: `docs/app/spec.md`
(planned). Reframes the scanner for a merchant demo: B's viewing key is pasted
into a browser UI, sealed to the scanner's X25519 pubkey, and the scenario's
transactions land in a Fava ledger.

- [ ] scanner: add `fee` to unshield events (Rust handle, `wire.ts`,
      `decrypt.ts`, tests, spec). The unshield event currently exposes only
      `value` (= base + fee); the merchant ledger needs the split to post net
      to `Assets:Bank:EOA2` and fee to `Expenses:Fees`.
- [ ] scanner: add `pubkey` op (wire, handle, server, tests). Returns the
      scanner's X25519 public key so the browser can seal the viewing key to
      it; only meaningful for the `sealedbox` KeyService.
- [ ] scanner: add `sealedbox` KeyService (X25519 + HKDF-SHA256 +
      XChaCha20-Poly1305 via `@noble/curves`/`@noble/ciphers`/`@noble/hashes`).
      Boot-time X25519 keypair; `decrypt` opens the sealed VK. Keep `local` for
      tests; `nitro` stays a future note.
- [ ] testbed: add `scripts/wallets.sh` (install → chain → deploy →
      create-wallets → export-keys, then print each wallet's address +
      shareable viewing key). `create-wallets` requires a deployment, so this
      must run the full setup first.
- [ ] testbed: product memos on the 5 transfers in `scripts/scenario.sh`
      (coffee-shop catalog: `2x Espresso`, `1x Latte, 1x Croissant, 1x Orange
      Juice`, `2x Sandwich, 2x Coffee`, `1x Cake, 3x Latte`, `Catering: 10x
      Coffee, 10x Pastry`). Shield/unshield have no on-chain memo.
- [ ] app: scaffold `app/` crate (actix-web, actix-files, alloy, tokio,
      serde, anyhow, base64, chrono, bigdecimal, `railgun-scanner-handle` path
      dep). Crate lives at repo root `./app`.
- [ ] app: scanner client thread (pubkey/init/scan over the Unix socket).
      Dedicated thread + mpsc channel; the sync handle is called from async
      actix/alloy code.
- [ ] app: alloy listener (backfill `deploymentBlock → latest` then WS
      subscribe; filter by proxy address; group logs by block; fetch block
      timestamps for the Beancount dates). Starts after scanner `init`.
- [ ] app: Beancount merchant ledger writer (merchant model, dedup by
      `txHash:logIndex:hash:direction`, atomic temp+rename rewrite). Wallet B
      only: incoming transfers → `Income:Sales`; unshield → bank + fee; change
      notes (`outputType=2`) excluded.
- [ ] app: actix-web UI + API (`/`, `/api/scanner-pubkey`, `/api/init`,
      `/api/status`). The UI pastes the VK; JS seals it and POSTs the
      ciphertext to `/api/init`, which forwards to the scanner.
- [ ] app: browser seal bundle (esbuild + noble) + static assets. Builds
      `seal.ts` → `static/seal.js`; serves `index.html`.
- [ ] docker: `app/Dockerfile` (esbuild static bundle + Rust multi-stage;
      build context = repo root to include `scanner/crates/handle`).
- [ ] docker: repo-root `docker-compose.yml` (`scanner-b` with
      `KEY_SERVICE=sealedbox` + socket volume, `app` on `host.docker.internal`
      + `.state:ro` + `ledger` volumes port 8080, `fava` waits for the ledger
      port 5000).
- [ ] docs: write `docs/app/spec.md` (architecture, sealed-box envelope,
      event→Beancount mapping, demo flow, verification).
- [ ] docs: update `docs/AGENTS.md` table + this backlog; update
      `docs/scanner/spec.md` (fee, pubkey, sealedbox) and
      `docs/testbed/spec.md` (wallets.sh, memos).
- [ ] verify: end-to-end run (`wallets.sh` → compose up → paste VK →
      `scenario.sh` → refresh Fava) + `bean-check ledger.beancount`; Fava
      shows 5 sales + 1 withdrawal, `Assets:Railgun:B` = 185 TST.

## Done

- [x] scanner: zod wire types + no `as` casts in production code. `wire.ts`
      defines all request/response types with zod; `server.ts` parses via
      `RequestSchema.safeParse`; `keys.ts` validates the shareable key. The
      only production cast left is the intentional structural cast to the
      engine's `TokenDataGetter` class (not a wire type).
- [x] Test bed: local Anvil chain + fresh RAILGUN V2 deployment + scenario
      (shield, 5x A->B transfers, unshield), steps/keys export
- [x] Scanner: stateless trial-decryption service, one Unix socket per viewing
      key, with Rust handle (protocol + acceptance tests)
