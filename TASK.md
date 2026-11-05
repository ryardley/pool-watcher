# Task: pool-watcher — RAILGUN transaction-history test bed (NixOS, devenv, Anvil)

This replaces all earlier versions of these instructions and all amendments.

## Principle

Build the simplest thing that works. This is a demo test bed, not production code:
- Avoid abstractions, frameworks and defensive code beyond what is needed for it to run reliably.
- When in doubt, choose fewer files and fewer steps.
- Record anything surprising in `NOTES.md` (short bullet points).

## Goal

1. Run a local Anvil chain with a fresh RAILGUN V2 deployment.
2. Run a scenario:
   - a public EOA shields an ERC-20 to RAILGUN wallet A;
   - A sends several private transfers to RAILGUN wallet B;
   - B unshields part of its balance to a public EOA.
3. Record what happened, and export the keys needed to scan the history later. Scanning and reassembling the history is not part of this task.

**Out of scope**
- Private POI (PPOI) entirely.
- Mainnet forks.
- Broadcasters/relayers.
- NFTs.
- View-only wallets, history scanning and event decryption.

## Environment

**Platform.** NixOS with devenv. The repo root contains `devenv.nix`, `devenv.yaml`, `devenv.lock` and an `.envrc`.

**`.envrc`.** Use the one `devenv init` generates (it contains `use devenv`). Entering the folder with direnv loads the tools. It must not install packages or run any project command.

**`devenv.nix`.** Tools only. No install hooks, no `enterShell` commands that touch the project, no `processes`.

```nix
{ pkgs, ... }:
{
  packages = [ pkgs.foundry pkgs.jq ];   # foundry provides anvil and cast
  languages.javascript = {
    enable = true;
    package = pkgs.nodejs_20;            # node + npm
    yarn.enable = true;                  # contracts repo uses a yarn v1 lockfile
    # leave npm.install / yarn.install at their defaults (off)
  };
}
```

**Option and package checks**
- Verify the option names against the pinned devenv version.
- If `pkgs.foundry` does not exist in the pinned nixpkgs, add the foundry.nix flake as a devenv input.
- Confirm `yarn --version` is 1.x.

**Installing packages** happens only in `scripts/install.sh`.

**Native module on NixOS**
- `leveldown` is the only expected native module. Its tarball ships prebuilt binaries, which normally load fine with the nix-store Node.
- Only if they fail to load:
  - add `python3`, `gcc` and `gnumake` to `packages`;
  - set `env.npm_config_nodedir = "${pkgs.nodejs_20}";`;
  - rebuild it from source.

## Repo layout

```
pool-watcher/
  .envrc  devenv.nix  devenv.yaml  devenv.lock
  .env                      # test mnemonics MNEMONIC_A, MNEMONIC_B (test values only)
  README.md  NOTES.md  .gitignore
  vendor/contract/          # git submodule: github.com/Railgun-Privacy/contract, pinned commit
  scripts/
    install.sh  chain.sh  stop-chain.sh  deploy.sh  setup.sh
    op.sh  scenario.sh  reset.sh
  demo/                     # TypeScript, ethers v6, npm
    package.json  package-lock.json  tsconfig.json
    src/
      lib.ts                # config, engine start/stop, gas details, sync helpers, wallet loading, step recording
      ops/
        create-wallets.ts  fund.ts  shield.ts  transfer.ts  unshield.ts
        balances.ts  export-keys.ts
  .cache/circuits/          # circuit artifacts (gitignored)
  .state/                   # chain + engine state (gitignored)
  out/                      # steps, keys (gitignored)
```

Put shared TypeScript code in a single `lib.ts` unless it grows past a few hundred lines.

## Where things are stored

| Location | Contents | Size | Removed by |
|---|---|---|---|
| `.cache/circuits/` | Circuit artifacts (wasm/zkey) downloaded from IPFS by the wallet SDK on first proof, via the `ArtifactStore`. | Largest download. Docs say 50 MB+ for the full set; only the variants used are fetched. | Manual `rm -rf .cache` only |
| `vendor/contract/node_modules/` | Contracts dependencies, including `railgun-circuit-test-artifacts` (an IPFS tarball). | Large | Manual only |
| `demo/node_modules/` | SDK, snarkjs, leveldown, etc. | Large | Manual only |
| `~/.cache/hardhat-nodejs/` | solc compiler downloaded by Hardhat on first compile. | Medium | Never by these scripts |
| `~/.npm`, yarn cache | Package-manager caches. | Varies | Never by these scripts |
| `/nix/store` | Tools, kept alive by devenv's GC roots in `.devenv/`. | Varies | Nix GC only |
| `.state/` | `anvil.json` (chain state), `anvil.pid`, `anvil.log`, `deployment.json`, `wallets.json`, engine DB `db-testbed/`. | Small | `scripts/reset.sh` |
| `out/` | `steps.jsonl`, `keys.json`. | Small | `scripts/reset.sh` |

**Rules**
- Big dependencies download once. No script deletes `node_modules`, `.cache/` or `.devenv/`.
- `PW_CACHE_DIR` may override the location of `.cache/` (one line in `lib.ts`).

## Background facts

**Contracts repo**
- Hardhat 2.x, ethers v5, yarn v1.
- `deploy:test` deploys everything to a local node: `npx hardhat deploy:test --network localhost`.

**Expected addresses.** Expected on a fresh chain from the default dev mnemonic. Anvil and Hardhat share the mnemonic and chain ID 31337, so the addresses should match the docs and shared-models:

| Contract | Address |
|---|---|
| proxy | 0x610178dA211FEF7D417bC0e6FeD39F05609AD788 |
| relayAdapt | 0x0355B7B8cb128fA5692729Ab3AAa199C1753f726 |
| weth9 | 0x8198f5d8F8CfFE8f9C413d98a0A55aEB8ab9FbB7 |
| testERC20 | 0x202CCe504e04bEd6fC0521238dDf04Bc9E8E15aB |

**SDK versions.** Pin the docs' recommended versions in `demo/`:
- `@railgun-community/wallet@10.4.0`
- `@railgun-community/shared-models@7.6.1`
- `ethers@6.13.1`
- `snarkjs`
- `leveldown`

Use `NetworkName.Hardhat`. Its config should match the addresses above with deployment block 0.

**Operations**
- Shield needs no proof.
- Transfers and unshields need Groth16 proofs made locally with snarkjs. The contract verifies them against verification keys (vkeys) registered per circuit shape.

**Viewing keys.** A wallet's viewing key decrypts its note data but cannot spend. It is what a later scanner will use, which is why this task exports it.

**Reference code.** `https://docs.railgun.org/developer-guide/llms-full.txt` contains working examples for:
- engine start;
- the `ArtifactStore`;
- the prover;
- `loadProvider`;
- shield, transfer and unshield;
- gas-detail helpers.

Follow it.

**Engine integration tests.** Repo `Railgun-Community/engine`, script `test-hardhat-V2`. They run against this same deployment. If something on the Hardhat network does not work through the wallet SDK, look at how those tests do it.

## Phase 0: checks before writing demo code

Write findings to `NOTES.md`. Keep this short.

1. **Verification keys.** Read the `deploy:test` task and find which vkeys it registers: from `railgun-circuit-test-artifacts`, or production.
   - The wallet SDK proves with production artifacts. With test vkeys, every transfer and unshield reverts.
   - If test vkeys are registered, plan a small script, run from `deploy.sh` after deploying, that registers production vkeys as the owner. Reuse the contracts repo's own vkey-formatting helper.
2. **Test token.** Does the deploy give account 0 a test ERC-20 balance? If not, how can it get one (mint, or deploy a minimal ERC-20 from the demo)? Also note its `decimals()`.
3. **Network config.** Print `NETWORK_CONFIG[NetworkName.Hardhat]` from the installed shared-models and confirm the addresses and chain ID.
4. **Sync on Hardhat.** Check how the wallet SDK syncs on Hardhat. There is no indexer, so it should fall back to RPC log scanning. Note what happens.

STOP and report if vkeys cannot be made to match, or if `NetworkName.Hardhat` is missing.

## Scripts

All scripts use bash with `set -euo pipefail` and work from any directory. They assume the devenv tools are on `PATH` (direnv or `devenv shell`).

**Idempotency.** Every setup script is idempotent: it checks first, prints `skip: <reason>`, and exits 0 if there is nothing to do.

**`install.sh`**
- `git submodule update --init`
- In `vendor/contract`: `yarn install --frozen-lockfile`, then compile. Skip each when it is already done: `node_modules` exists and `yarn.lock` has not changed since the last install. Store the lockfile's sha256 in `node_modules/.installed`; do the same for compile.
- In `demo`: `npm ci`, using the same lockfile-hash check.

**`chain.sh`**
- If `cast chain-id` answers 31337, skip.
- Otherwise start `anvil --state .state/anvil.json` in the background, logging to `.state/anvil.log`, and write its pid to `.state/anvil.pid`.
- Wait until it answers.
- Because `--state` reloads the previous chain, a restart keeps the deployment and all transactions.

**`stop-chain.sh`**
- Kill the pid in `.state/anvil.pid` and remove the file.
- Anvil writes its state on exit.

**`deploy.sh`**
- If `.state/deployment.json` exists and `cast code <proxy>` is non-empty, skip.
- Otherwise:
  1. Delete `.state/wallets.json`, `.state/db-*` and `out/`. They belong to a previous chain.
  2. Run `deploy:test`.
  3. Save the printed addresses and the proxy's deployment block to `.state/deployment.json`.
  4. Register vkeys if Phase 0 requires it.

**`setup.sh`**
```
install.sh → chain.sh → deploy.sh → op.sh create-wallets --names A,B → op.sh fund --eoa 0 --amount 1000
```

**`op.sh <name> [flags]`**
- Runs `npx tsx src/ops/<name>.ts [flags]` in `demo/`.
- Uses `.state/db-testbed` unless `DB_PATH` is set.
- Op scripts must run one at a time, because the engine DB is locked per process.

**`scenario.sh`**
- The master script. Parameters at the top; nothing but a sequence of `op.sh` calls. See "Scenario".

**`reset.sh`**
- Runs `stop-chain.sh`, then deletes `.state/` and `out/`.
- Never touches `.cache/` or `node_modules`.

## Op scripts

**Common pattern for each op**
1. Parse flags with `node:util` `parseArgs`.
2. Start the engine.
3. Sync the wallets involved: `refreshBalances`, then wait for the scan to finish.
4. Do the one thing.
5. Wait for the receipt.
6. Sync again.
7. Print a short human-readable summary, including balances before and after.
8. Append one JSON line to `out/steps.jsonl` (chain-changing ops only).
9. Stop the engine.

Exit non-zero on any error. Keep each op file short.

**Engine setup in `lib.ts`**
- Call: `startRailgunEngine("poolwatcher", leveldown(DB_PATH), debug, artifactStore(.cache/circuits), false, false, [], [], verbose)`.
  - The POI node list is empty and POI is never touched.
  - If the SDK rejects an empty list, pass `["http://127.0.0.1:1"]` and note it.
- Prover: `getProver().setSnarkJSGroth16(groth16)`.
- Provider: `loadProvider({ chainId: 31337, providers: [{ provider: RPC_URL, priority: 1, weight: 2 }] }, NetworkName.Hardhat, 10_000)`.
- Encryption key: a fixed 32-byte hex constant.
- If received funds turn out not to be spendable because the SDK applies POI on Hardhat, add the smallest possible runtime override that makes POI not required for Hardhat. Note it.

**Accounts.** Anvil account 0 shields. Account 1 submits proved transactions. Account 2 receives the unshield.

**The ops**

| Op | Flags | What it does |
|---|---|---|
| `create-wallets` | `--names A,B` | Creates wallets from env mnemonics `MNEMONIC_A` and `MNEMONIC_B` (from `.env`, creation block 0). Saves ids and 0zk addresses to `.state/wallets.json`. Skips wallets already there. |
| `fund` | `--eoa 0 --amount N` | Ensures the EOA holds at least N test tokens; tops up if needed. Saves the token address to `.state/wallets.json` if a demo token was deployed. |
| `shield` | `--from-eoa 0 --to A --amount N` | Approve if needed. Shield private key = `keccak256(signMessage(getShieldPrivateKeySignatureMessage()))`. Then `gasEstimateForShield` → `populateShield` → send. Report the shield fee the contract applied. |
| `transfer` | `--from A --to B --amount N [--memo text] [--show-sender] [--submitter 1]` | `gasEstimateForUnprovenTransfer` → `generateTransferProof` → `populateProvedTransfer` (same params as the proof) → send from the submitter. Uses `sendWithPublicWallet = true` and no broadcaster fee. |
| `unshield` | `--from B --to-eoa 2 --amount N [--submitter 1]` | `gasEstimateForUnprovenUnshield` → `generateUnshieldProof` → `populateProvedUnshield` → send. Report the unshield fee the contract applied. |
| `balances` | `[--wallets A,B] [--eoas 2]` | Print private spendable balances and public token balances. |
| `export-keys` | — | Write `out/keys.json`. See below. |

**Gas details.** Build them from `provider.getFeeData()` using the EVM gas type in the Hardhat network config. Follow the docs' helper.

**Amounts.** Accept amounts in whole tokens and scale them by the token's decimals.

**Step record format.** One JSON object per line in `out/steps.jsonl`:

```json
{ "kind": "shield|transfer|unshield", "txHash": "0x…", "block": 0, "from": "…", "to": "…", "amount": "base units", "fee": "base units or null", "memo": "…" }
```

**`out/keys.json` format**

```json
{
  "A": {
    "railgunAddress": "0zk…",
    "shareableViewingKey": "…",
    "viewingPrivateKey": "0x…",
    "creationBlock": 0
  },
  "B": { "...": "same fields" }
}
```

- `shareableViewingKey` comes from `getWalletShareableViewingKey`.
- `viewingPrivateKey` comes from `getRailgunWalletPrivateViewingKey`, hex-encoded.
- These are test keys; writing them to disk is fine.

## Scenario (`scripts/scenario.sh`)

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ -f ../out/steps.jsonl ]]; then
  echo "scenario already run on this chain; run scripts/reset.sh first" >&2
  exit 1
fi

./setup.sh

./op.sh shield --from-eoa 0 --to A --amount 1000

i=0
for amt in 10 25 40 60 100; do
  flag=""; (( i % 2 == 0 )) && flag="--show-sender"
  ./op.sh transfer --from A --to B --amount "$amt" --memo "transfer #$i" $flag
  i=$((i+1))
done

./op.sh unshield --from B --to-eoa 2 --amount 50
./op.sh balances --wallets A,B --eoas 2
./op.sh export-keys
```

**Ordering detail.** Each transfer op syncs A before proving, so A's change note from the previous transfer is known. This is why every op syncs at its start.

## Done when

- A fresh clone with direnv allowed runs `scripts/scenario.sh` end to end.
- After the run, these exist and are consistent with each other:
  - `out/steps.jsonl` (one line per shield, transfer and unshield);
  - `out/keys.json`;
  - `.state/deployment.json` (proxy address, deployment block);
  - the chain in `.state/anvil.json`.
- The `balances` output matches the scenario arithmetic: shield amount minus shield fee, less transfers, plus unshield minus unshield fee. Use the fee amounts the contract applied rather than recomputing them.
- Running `scripts/setup.sh` a second time prints only `skip:` lines.
- `scripts/reset.sh && scripts/scenario.sh` works without re-downloading circuits or reinstalling packages.
- After `scripts/stop-chain.sh` and `scripts/chain.sh`, the chain still holds the full history.
- `README.md` lists:
  - the commands: `scenario.sh`, `op.sh` examples, `chain.sh` / `stop-chain.sh`, `reset.sh`;
  - the storage table above;
  - a short "Scanning the history later" note naming what a scanner needs: RPC URL (`http://127.0.0.1:8545`), proxy address and deployment block from `.state/deployment.json`, and the `viewingPrivateKey` or `shareableViewingKey` from `out/keys.json`.
- `NOTES.md` holds the Phase 0 findings and any workarounds.
