# pool-watcher

A local RAILGUN V2 test bed: Anvil + a fresh contract deployment + a scripted
shield / private-transfer / unshield scenario. It records the transactions and
exports the viewing keys needed to scan the history later.

## Requirements

NixOS with devenv and direnv. Allow direnv in the repo root (`direnv allow`),
or enter the shell with `devenv shell`.

## Commands

Run the whole scenario from a clean state:

```sh
scripts/scenario.sh
```

`scenario.sh` runs `setup.sh` (install, chain, deploy, create wallets, fund),
then shields 1000 test tokens to wallet A, sends five private transfers from A
to B, unshields 50 from B to EOA 2, prints balances, and exports keys.

Individual operations (the chain must be running):

```sh
scripts/op.sh shield   --from-eoa 0 --to A --amount 1000
scripts/op.sh transfer --from A --to B --amount 10 --memo "hello" --show-sender
scripts/op.sh unshield --from B --to-eoa 2 --amount 50
scripts/op.sh balances --wallets A,B --eoas 2
scripts/op.sh export-keys
scripts/op.sh fund     --eoa 0 --amount 1000
scripts/op.sh create-wallets --names A,B
```

Chain lifecycle:

```sh
scripts/chain.sh        # start anvil, loading .state/anvil.json if present
scripts/stop-chain.sh   # SIGINT anvil so it dumps state, then remove the pid file
scripts/reset.sh        # stop the chain, delete .state/ and out/
```

`setup.sh` is idempotent: a second run prints only `skip:` lines.

## Storage

| Location | Contents | Size | Removed by |
|---|---|---|---|
| `.cache/circuits/` | Circuit artifacts (wasm/zkey) downloaded from IPFS by the wallet SDK on first proof, via the `ArtifactStore`. | Largest download. | Manual `rm -rf .cache` only |
| `.cache/vkeys/` | Production vkey JSONs fetched from IPFS by the vkey-registration step. | Small. | Manual `rm -rf .cache` only |
| `vendor/contract/node_modules/` | Contracts dependencies. | Large | Manual only |
| `demo/node_modules/` | SDK, snarkjs, leveldown, etc. | Large | Manual only |
| `~/.cache/hardhat-nodejs/` | solc compiler downloaded by Hardhat on first compile. | Medium | Never by these scripts |
| `~/.npm`, yarn cache | Package-manager caches. | Varies | Never by these scripts |
| `/nix/store` | Tools, kept alive by devenv's GC roots in `.devenv/`. | Varies | Nix GC only |
| `.state/` | `anvil.json` (chain state), `anvil.pid`, `anvil.log`, `deployment.json`, `wallets.json`, engine DB `db-testbed/`. | Small | `scripts/reset.sh` |
| `out/` | `steps.jsonl`, `keys.json`. | Small | `scripts/reset.sh` |

Big dependencies download once. No script deletes `node_modules`, `.cache/` or
`.devenv/`. `PW_CACHE_DIR` overrides the `.cache/` location; `PW_RPC_URL`
overrides the RPC URL; `DB_PATH` overrides the engine database path.

## Scanning the history later

A scanner needs:

- RPC URL: `http://127.0.0.1:8545`
- proxy address and deployment block: `.state/deployment.json`
- `viewingPrivateKey` or `shareableViewingKey` for each wallet: `out/keys.json`

The chain state is in `.state/anvil.json`; restart it with `scripts/chain.sh`
to keep the full history.
