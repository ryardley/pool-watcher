# Notes

## Phase 0 findings

- **Verification keys.** `deploy:test` registers test vkeys from
  `@railgun-community/circuit-artifacts`, but that tarball
  (`https://npm.railgun.org/railgun-community-circuit-artifacts-0.0.1.tgz`)
  returns HTTP 525. The contracts repo is patched by `install.sh`
  (`scripts/contracts-no-test-artifacts.patch`) to drop the dependency and make
  `loadAllArtifacts` a no-op. `demo/src/register-vkeys.ts` then registers
  production vkeys for every non-POI variant in wallet 10.4.0's
  `artifact-v2-hashes.json` (54 variants; the task said 56).
- **Vkey path.** The SDK's actual IPFS path is `<NNxNN>/vkey.json` under
  master hash `QmeBrG7pii1qTqsn7rusvDiqXopHPjCT9gR4PsmW7wXqZq`, not
  `<NNxNN>/circuits/vkey.json` (the `artifacts-v2.1/` prefix is only the local
  cache directory). Vkeys are cached in `.cache/vkeys/`.
- **Vkey registration gas.** `eth_estimateGas` underestimates
  `delegator.callContract(proxy, setVerificationKey, 0)` (~87k instead of
  ~1.2M), so the inner call runs out of gas and `callContract` returns false
  while the outer transaction still succeeds. The script sends with an explicit
  `gasLimit: 5_000_000` and checks for the `VerifyingKeySet` event.
- **Test token.** `TestERC20` has an open `mint(address,uint256)` and 18
  decimals. `fund` mints the shortfall; no demo token is needed.
- **Network config.** `NetworkName.Hardhat` exists (chain id 31337, Type2 gas,
  deployment block 0). The proxy address matches the expected table, but the
  pinned contracts commit deploys different `relayAdapt`, `weth9` and
  `testERC20` addresses than the table. The scenario reads `testERC20` from
  `.state/deployment.json` and only uses the proxy, so this does not matter.
- **V3.** The SDK's Hardhat config advertises V3 addresses that the pinned
  contracts repo does not deploy. `lib.ts` disables V3 at runtime
  (`supportsV3 = false`) in both the top-level and the wallet SDK's nested
  `@railgun-community/shared-models` copy.
- **Sync.** Quick sync returns no events for Hardhat; the engine falls back to
  full RPC `getLogs` scanning from block 0. Confirmed at runtime.
- **Node 22.** `yarn install` fails the `eslint-plugin-jsdoc` engine check on
  Node 22; `install.sh` uses `--ignore-engines`.
- **Engine shutdown.** `stopRailgunEngine` leaves the polling provider/timers
  alive, so op scripts call `process.exit(0)` after stopping the engine.
- **Resolved versions.** `snarkjs@0.7.6`, `leveldown@6.1.1`.
- **Viewing key format.** `getWalletShareableViewingKey` returns a hex string
  (not base64).
- **Deploy output parsing.** The last field of Hardhat's `DEPLOY CONFIG` object
  has no trailing comma; `deploy.sh` parses addresses with an optional suffix.
