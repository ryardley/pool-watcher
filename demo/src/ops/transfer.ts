import { parseArgs } from 'node:util';
import {
  gasEstimateForUnprovenTransfer,
  generateTransferProof,
  populateProvedTransfer,
} from '@railgun-community/wallet';
import {
  ENCRYPTION_KEY,
  NETWORK,
  TXID_VERSION,
  batchMinGasPrice,
  eoa,
  erc20Recipient,
  formatAmount,
  gasDetails,
  loadWallet,
  privateBalance,
  readDeployment,
  recordStep,
  startEngine,
  stopEngine,
  syncWallets,
  toBaseUnits,
  tokenDecimals,
} from '../lib.js';

const { values } = parseArgs({
  options: {
    from: { type: 'string' },
    to: { type: 'string' },
    amount: { type: 'string' },
    memo: { type: 'string' },
    'show-sender': { type: 'boolean', default: false },
    submitter: { type: 'string', default: '1' },
  },
});

if (!values.from || !values.to || !values.amount) {
  throw new Error('--from, --to and --amount are required');
}

await startEngine();
const from = await loadWallet(values.from);
const to = await loadWallet(values.to);
const deployment = readDeployment();
const amount = await toBaseUnits(deployment.testERC20, values.amount);
const decimals = await tokenDecimals(deployment.testERC20);
const recipient = erc20Recipient(
  deployment.testERC20,
  amount,
  to.railgunAddress,
);
const submitter = eoa(Number(values.submitter));

await syncWallets([from.id, to.id]);
const fromBefore = await privateBalance(from.id, deployment.testERC20);
const toBefore = await privateBalance(to.id, deployment.testERC20);
const original = await gasDetails(0n, true);
const { gasEstimate } = await gasEstimateForUnprovenTransfer(
  TXID_VERSION,
  NETWORK,
  from.id,
  ENCRYPTION_KEY,
  values.memo,
  [recipient],
  [],
  original,
  undefined,
  true,
);
const details = await gasDetails(gasEstimate, true);
const minGasPrice = batchMinGasPrice(details);

await generateTransferProof(
  TXID_VERSION,
  NETWORK,
  from.id,
  ENCRYPTION_KEY,
  values['show-sender'],
  values.memo,
  [recipient],
  [],
  undefined,
  true,
  minGasPrice,
  (progress) => {
    const percent = Math.round(progress);
    if (percent % 20 === 0) console.log(`proof ${percent}%`);
  },
);

const { transaction } = await populateProvedTransfer(
  TXID_VERSION,
  NETWORK,
  from.id,
  values['show-sender'],
  values.memo,
  [recipient],
  [],
  undefined,
  true,
  minGasPrice,
  details,
);
const tx = await submitter.sendTransaction(transaction);
const receipt = await tx.wait();
if (!receipt) throw new Error('transfer transaction was not mined');

recordStep({
  kind: 'transfer',
  txHash: receipt.hash,
  block: receipt.blockNumber,
  from: from.railgunAddress,
  to: to.railgunAddress,
  amount: amount.toString(),
  fee: null,
  memo: values.memo ?? null,
});

await syncWallets([from.id, to.id]);
const fromAfter = await privateBalance(from.id, deployment.testERC20);
const toAfter = await privateBalance(to.id, deployment.testERC20);
console.log(
  `transferred ${formatAmount(amount, decimals)} from ${values.from} to ${values.to}${values.memo ? ` (memo: ${values.memo})` : ''}; ${values.from} private ${formatAmount(fromBefore, decimals)} -> ${formatAmount(fromAfter, decimals)}; ${values.to} private ${formatAmount(toBefore, decimals)} -> ${formatAmount(toAfter, decimals)}`,
);
await stopEngine();
process.exit(0);
