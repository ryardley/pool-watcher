import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import leveldown from 'leveldown';
import { groth16 } from 'snarkjs';
import {
  ArtifactStore,
  createRailgunWallet,
  getProver,
  getRailgunWalletPrivateViewingKey,
  getShieldPrivateKeySignatureMessage,
  getWalletShareableViewingKey,
  loadProvider,
  loadWalletByID,
  refreshBalances,
  startRailgunEngine,
  stopRailgunEngine,
  walletForID,
} from '@railgun-community/wallet';
import {
  calculateGasPrice,
  EVMGasType,
  getEVMGasTypeForTransaction,
  NETWORK_CONFIG,
  NetworkName,
  TXIDVersion,
  type FallbackProviderJsonConfig,
  type RailgunERC20AmountRecipient,
  type TransactionGasDetails,
} from '@railgun-community/shared-models';
import {
  Contract,
  HDNodeWallet,
  Interface,
  JsonRpcProvider,
  formatUnits,
  keccak256,
  parseUnits,
  type TransactionReceipt,
} from 'ethers';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const ROOT = path.resolve(HERE, '../..');
export const RPC_URL = process.env.PW_RPC_URL ?? 'http://127.0.0.1:8545';
export const CACHE_DIR = process.env.PW_CACHE_DIR ?? path.join(ROOT, '.cache');
export const DB_PATH = process.env.DB_PATH ?? path.join(ROOT, '.state/db-testbed');
export const STATE_DIR = path.join(ROOT, '.state');
export const OUT_DIR = path.join(ROOT, 'out');
export const NETWORK = NetworkName.Hardhat;
export const CHAIN = NETWORK_CONFIG[NETWORK].chain;
export const TXID_VERSION = TXIDVersion.V2_PoseidonMerkle;
export const ENCRYPTION_KEY = '11'.repeat(32);
export const ANVIL_MNEMONIC =
  'test test test test test test test test test test test junk';

export const provider = new JsonRpcProvider(RPC_URL);

const artifactStore = new ArtifactStore(
  async (p) => fs.promises.readFile(path.join(CACHE_DIR, 'circuits', p)),
  async (_dir, p, item) => {
    const full = path.join(CACHE_DIR, 'circuits', p);
    await fs.promises.mkdir(path.dirname(full), { recursive: true });
    await fs.promises.writeFile(full, item);
  },
  async (p) =>
    fs.promises
      .access(path.join(CACHE_DIR, 'circuits', p))
      .then(() => true)
      .catch(() => false),
);

const nodeRequire = createRequire(import.meta.url);

function disableV3(): void {
  // The pinned contracts repo has no V3 deployment, but the SDK's Hardhat
  // network config advertises V3 addresses. Disable V3 so loadProvider does
  // not call the missing token vault.
  (NETWORK_CONFIG[NETWORK] as { supportsV3: boolean }).supportsV3 = false;
  try {
    const walletMain = nodeRequire.resolve('@railgun-community/wallet');
    const nestedPath = path.join(
      path.dirname(walletMain),
      '../node_modules/@railgun-community/shared-models',
    );
    const nested = nodeRequire(nestedPath) as {
      NETWORK_CONFIG: Record<string, { supportsV3: boolean }>;
    };
    nested.NETWORK_CONFIG[NETWORK].supportsV3 = false;
  } catch {
    // No nested shared-models copy.
  }
}

export async function startEngine(): Promise<void> {
  disableV3();
  await startRailgunEngine(
    'poolwatcher',
    leveldown(DB_PATH),
    false,
    artifactStore,
    false,
    false,
    [],
    [],
    false,
  );
  getProver().setSnarkJSGroth16(groth16 as never);
  const config: FallbackProviderJsonConfig = {
    chainId: CHAIN.id,
    providers: [{ provider: RPC_URL, priority: 1, weight: 2 }],
  };
  await loadProvider(config, NETWORK, 10_000);
}

export async function stopEngine(): Promise<void> {
  await stopRailgunEngine();
}

export function eoa(index: number): HDNodeWallet {
  return HDNodeWallet.fromPhrase(
    ANVIL_MNEMONIC,
    undefined,
    `m/44'/60'/0'/0/${index}`,
  ).connect(provider);
}

export type Deployment = {
  proxy: string;
  delegator: string;
  testERC20: string;
  weth9: string;
  relayAdapt: string;
  deploymentBlock: number;
};

export function readDeployment(): Deployment {
  return JSON.parse(
    fs.readFileSync(path.join(STATE_DIR, 'deployment.json'), 'utf8'),
  ) as Deployment;
}

export type WalletRecord = { id: string; railgunAddress: string };
export type WalletsFile = Record<string, WalletRecord>;

export function readWallets(): WalletsFile {
  const file = path.join(STATE_DIR, 'wallets.json');
  if (!fs.existsSync(file)) return {};
  return JSON.parse(fs.readFileSync(file, 'utf8')) as WalletsFile;
}

export function writeWallets(wallets: WalletsFile): void {
  fs.mkdirSync(STATE_DIR, { recursive: true });
  fs.writeFileSync(
    path.join(STATE_DIR, 'wallets.json'),
    `${JSON.stringify(wallets, null, 2)}\n`,
  );
}

export async function loadWallet(name: string): Promise<WalletRecord> {
  const record = readWallets()[name];
  if (!record) {
    throw new Error(`wallet ${name} not found; run create-wallets first`);
  }
  await loadWalletByID(ENCRYPTION_KEY, record.id, false);
  return record;
}

export async function createWallet(
  name: string,
  mnemonic: string,
): Promise<WalletRecord> {
  const info = await createRailgunWallet(ENCRYPTION_KEY, mnemonic, {
    [NETWORK]: 0,
  });
  return { id: info.id, railgunAddress: info.railgunAddress };
}

export async function syncWallets(ids: string[]): Promise<void> {
  await refreshBalances(CHAIN, ids);
}

export async function privateBalance(
  walletId: string,
  tokenAddress: string,
): Promise<bigint> {
  const balances = await walletForID(walletId).getTokenBalances(
    TXID_VERSION,
    CHAIN,
    true,
  );
  for (const balance of Object.values(balances)) {
    if (
      balance.tokenData.tokenAddress.toLowerCase() ===
      tokenAddress.toLowerCase()
    ) {
      return balance.balance;
    }
  }
  return 0n;
}

export async function gasDetails(
  gasEstimate: bigint,
  sendWithPublicWallet = true,
): Promise<TransactionGasDetails> {
  const evmGasType = getEVMGasTypeForTransaction(
    NETWORK,
    sendWithPublicWallet,
  );
  const feeData = await provider.getFeeData();
  if (evmGasType === EVMGasType.Type2) {
    return {
      evmGasType,
      gasEstimate,
      maxFeePerGas: feeData.maxFeePerGas ?? 0n,
      maxPriorityFeePerGas: feeData.maxPriorityFeePerGas ?? 0n,
    };
  }
  return {
    evmGasType,
    gasEstimate,
    gasPrice: feeData.gasPrice ?? 0n,
  };
}

export function batchMinGasPrice(details: TransactionGasDetails): bigint {
  return calculateGasPrice(details);
}

export async function shieldPrivateKey(wallet: HDNodeWallet): Promise<string> {
  return keccak256(
    await wallet.signMessage(getShieldPrivateKeySignatureMessage()),
  );
}

export const ERC20_ABI = [
  'function balanceOf(address) view returns (uint256)',
  'function allowance(address,address) view returns (uint256)',
  'function approve(address,uint256) returns (bool)',
  'function mint(address,uint256)',
  'function decimals() view returns (uint8)',
];

export function tokenContract(
  address: string,
  signerOrProvider: HDNodeWallet | JsonRpcProvider = provider,
): Contract {
  return new Contract(address, ERC20_ABI, signerOrProvider);
}

export async function tokenDecimals(address: string): Promise<number> {
  return Number(await tokenContract(address).decimals());
}

export async function toBaseUnits(
  tokenAddress: string,
  amount: string,
): Promise<bigint> {
  return parseUnits(amount, await tokenDecimals(tokenAddress));
}

export function formatAmount(amount: bigint, decimals: number): string {
  return formatUnits(amount, decimals);
}

export function erc20Recipient(
  tokenAddress: string,
  amount: bigint,
  recipientAddress: string,
): RailgunERC20AmountRecipient {
  return { tokenAddress, amount, recipientAddress };
}

export type Step = {
  kind: 'shield' | 'transfer' | 'unshield';
  txHash: string;
  block: number;
  from: string;
  to: string;
  amount: string;
  fee: string | null;
  memo: string | null;
};

export function recordStep(step: Step): void {
  fs.mkdirSync(OUT_DIR, { recursive: true });
  fs.appendFileSync(
    path.join(OUT_DIR, 'steps.jsonl'),
    `${JSON.stringify(step)}\n`,
  );
}

const railgunArtifact = path.join(
  ROOT,
  'vendor/contract/artifacts/contracts/logic/RailgunSmartWallet.sol/RailgunSmartWallet.json',
);
const railgunInterface = new Interface(
  JSON.parse(fs.readFileSync(railgunArtifact, 'utf8')).abi,
);

function eventArg(
  receipt: TransactionReceipt,
  eventName: string,
): Record<string, unknown> | undefined {
  for (const log of receipt.logs) {
    try {
      const parsed = railgunInterface.parseLog(log);
      if (parsed?.name === eventName) {
        return parsed.args as unknown as Record<string, unknown>;
      }
    } catch {
      // Not a Railgun event.
    }
  }
  return undefined;
}

export function shieldFeeFromReceipt(receipt: TransactionReceipt): bigint {
  const args = eventArg(receipt, 'Shield');
  if (!args) throw new Error('Shield event not found in receipt');
  return (args.fees as bigint[])[0];
}

export function unshieldFeeFromReceipt(receipt: TransactionReceipt): bigint {
  const args = eventArg(receipt, 'Unshield');
  if (!args) throw new Error('Unshield event not found in receipt');
  return args.fee as bigint;
}

export async function exportKeys(): Promise<void> {
  const wallets = readWallets();
  const out: Record<string, unknown> = {};
  for (const [name, record] of Object.entries(wallets)) {
    await loadWalletByID(ENCRYPTION_KEY, record.id, false);
    const shareableViewingKey = await getWalletShareableViewingKey(record.id);
    const viewingPrivateKey = getRailgunWalletPrivateViewingKey(record.id);
    out[name] = {
      railgunAddress: record.railgunAddress,
      shareableViewingKey,
      viewingPrivateKey: `0x${Buffer.from(viewingPrivateKey).toString('hex')}`,
      creationBlock: 0,
    };
  }
  fs.mkdirSync(OUT_DIR, { recursive: true });
  fs.writeFileSync(
    path.join(OUT_DIR, 'keys.json'),
    `${JSON.stringify(out, null, 2)}\n`,
  );
}
