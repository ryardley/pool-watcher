import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { Contract, HDNodeWallet, Interface, JsonRpcProvider } from 'ethers';
import {
  ANVIL_MNEMONIC,
  CACHE_DIR,
  RPC_URL,
  STATE_DIR,
} from './lib.js';

const require = createRequire(import.meta.url);
const walletMain = require.resolve('@railgun-community/wallet');
const hashesPath = path.join(
  path.dirname(walletMain),
  'services/artifacts/json/artifact-v2-hashes.json',
);
const hashes = JSON.parse(fs.readFileSync(hashesPath, 'utf8')) as Record<
  string,
  unknown
>;
const variants = Object.keys(hashes).filter(
  (variant) => !variant.startsWith('POI'),
);

const MASTER_IPFS_HASH = 'QmeBrG7pii1qTqsn7rusvDiqXopHPjCT9gR4PsmW7wXqZq';
const GATEWAYS = ['https://ipfs-lb.com/ipfs', 'https://ipfs.io/ipfs'];

const deployment = JSON.parse(
  fs.readFileSync(path.join(STATE_DIR, 'deployment.json'), 'utf8'),
) as { proxy: string; delegator: string };

const provider = new JsonRpcProvider(RPC_URL);
const signer = HDNodeWallet.fromPhrase(
  ANVIL_MNEMONIC,
  undefined,
  "m/44'/60'/0'/0/0",
).connect(provider);

const VERIFIER_ABI = [
  'function setVerificationKey(uint256 _nullifiers, uint256 _commitments, (string artifactsIPFSHash, (uint256 x, uint256 y) alpha1, (uint256[2] x, uint256[2] y) beta2, (uint256[2] x, uint256[2] y) gamma2, (uint256[2] x, uint256[2] y) delta2, (uint256 x, uint256 y)[] ic) _verifyingKey)',
  'event VerifyingKeySet(uint256 nullifiers, uint256 commitments, (string artifactsIPFSHash, (uint256 x, uint256 y) alpha1, (uint256[2] x, uint256[2] y) beta2, (uint256[2] x, uint256[2] y) gamma2, (uint256[2] x, uint256[2] y) delta2, (uint256 x, uint256 y)[] ic) verifyingKey)',
];
const DELEGATOR_ABI = [
  'function callContract(address _contract, bytes _data, uint256 _value) returns (bool success, bytes returnData)',
];

const verifierInterface = new Interface(VERIFIER_ABI);
const delegator = new Contract(deployment.delegator, DELEGATOR_ABI, signer);

// Verbatim copy of formatVKey from the contracts repo
// (helpers/logic/artifacts.ts:69 at the pinned commit).
function formatVKey(vkey: {
  vk_alpha_1: string[];
  vk_beta_2: string[][];
  vk_gamma_2: string[][];
  vk_delta_2: string[][];
  IC: string[][];
}) {
  return {
    artifactsIPFSHash: '',
    alpha1: {
      x: BigInt(vkey.vk_alpha_1[0]),
      y: BigInt(vkey.vk_alpha_1[1]),
    },
    beta2: {
      x: [BigInt(vkey.vk_beta_2[0][1]), BigInt(vkey.vk_beta_2[0][0])],
      y: [BigInt(vkey.vk_beta_2[1][1]), BigInt(vkey.vk_beta_2[1][0])],
    },
    gamma2: {
      x: [BigInt(vkey.vk_gamma_2[0][1]), BigInt(vkey.vk_gamma_2[0][0])],
      y: [BigInt(vkey.vk_gamma_2[1][1]), BigInt(vkey.vk_gamma_2[1][0])],
    },
    delta2: {
      x: [BigInt(vkey.vk_delta_2[0][1]), BigInt(vkey.vk_delta_2[0][0])],
      y: [BigInt(vkey.vk_delta_2[1][1]), BigInt(vkey.vk_delta_2[1][0])],
    },
    ic: vkey.IC.map((point) => ({
      x: BigInt(point[0]),
      y: BigInt(point[1]),
    })),
  };
}

async function fetchVKey(variant: string): Promise<Parameters<typeof formatVKey>[0]> {
  const cacheFile = path.join(CACHE_DIR, 'vkeys', `${variant}.json`);
  if (fs.existsSync(cacheFile)) {
    return JSON.parse(fs.readFileSync(cacheFile, 'utf8'));
  }
  let lastError: unknown;
  for (const gateway of GATEWAYS) {
    try {
      const response = await fetch(
        `${gateway}/${MASTER_IPFS_HASH}/${variant}/vkey.json`,
      );
      if (!response.ok) {
        throw new Error(`${response.status} ${response.statusText}`);
      }
      const json = await response.json();
      fs.mkdirSync(path.dirname(cacheFile), { recursive: true });
      fs.writeFileSync(cacheFile, JSON.stringify(json));
      return json as Parameters<typeof formatVKey>[0];
    } catch (error) {
      lastError = error;
    }
  }
  throw new Error(`failed to fetch vkey for ${variant}: ${lastError}`);
}

for (const variant of variants) {
  const [nullifiers, commitments] = variant.split('x').map(Number);
  const vkey = await fetchVKey(variant);
  const data = verifierInterface.encodeFunctionData('setVerificationKey', [
    nullifiers,
    commitments,
    formatVKey(vkey),
  ]);
  const [success] = (await delegator.callContract.staticCall(
    deployment.proxy,
    data,
    0n,
  )) as [boolean, string];
  if (!success) {
    throw new Error(`setVerificationKey failed for ${variant}`);
  }
  const tx = await delegator.callContract(deployment.proxy, data, 0n, {
    gasLimit: 5_000_000n,
  });
  const receipt = await tx.wait();
  if (!receipt) throw new Error(`no receipt for ${variant}`);
  const registered = receipt.logs.some((log) => {
    try {
      return verifierInterface.parseLog(log)?.name === 'VerifyingKeySet';
    } catch {
      return false;
    }
  });
  if (!registered) {
    throw new Error(`VerifyingKeySet event missing for ${variant}`);
  }
  console.log(`vkey ${variant} registered`);
}

console.log(`registered ${variants.length} production vkeys`);
process.exit(0);
