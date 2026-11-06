import { parseArgs } from 'node:util';
import {
  gasEstimateForUnprovenUnshield,
  generateUnshieldProof,
  populateProvedUnshield,
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
  tokenContract,
  tokenDecimals,
  unshieldFeeFromReceipt,
} from '../lib.js';

const { values } = parseArgs({
  options: {
    from: { type: 'string' },
    'to-eoa': { type: 'string', default: '2' },
    amount: { type: 'string' },
    submitter: { type: 'string', default: '1' },
  },
});

if (!values.from || !values.amount) {
  throw new Error('--from and --amount are required');
}

await startEngine();
const from = await loadWallet(values.from);
const deployment = readDeployment();
const amount = await toBaseUnits(deployment.testERC20, values.amount);
const decimals = await tokenDecimals(deployment.testERC20);
const destination = eoa(Number(values['to-eoa']));
const recipient = erc20Recipient(
  deployment.testERC20,
  amount,
  destination.address,
);
const submitter = eoa(Number(values.submitter));

await syncWallets([from.id]);
const fromBefore = await privateBalance(from.id, deployment.testERC20);
const destinationBefore = (await tokenContract(
  deployment.testERC20,
).balanceOf(destination.address)) as bigint;
const original = await gasDetails(0n, true);
const { gasEstimate } = await gasEstimateForUnprovenUnshield(
  TXID_VERSION,
  NETWORK,
  from.id,
  ENCRYPTION_KEY,
  [recipient],
  [],
  original,
  undefined,
  true,
);
const details = await gasDetails(gasEstimate, true);
const minGasPrice = batchMinGasPrice(details);

await generateUnshieldProof(
  TXID_VERSION,
  NETWORK,
  from.id,
  ENCRYPTION_KEY,
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

const { transaction } = await populateProvedUnshield(
  TXID_VERSION,
  NETWORK,
  from.id,
  [recipient],
  [],
  undefined,
  true,
  minGasPrice,
  details,
);
const tx = await submitter.sendTransaction(transaction);
const receipt = await tx.wait();
if (!receipt) throw new Error('unshield transaction was not mined');
const fee = unshieldFeeFromReceipt(receipt);

recordStep({
  kind: 'unshield',
  txHash: receipt.hash,
  block: receipt.blockNumber,
  from: from.railgunAddress,
  to: destination.address,
  amount: amount.toString(),
  fee: fee.toString(),
  memo: null,
});

await syncWallets([from.id]);
const fromAfter = await privateBalance(from.id, deployment.testERC20);
const destinationAfter = (await tokenContract(
  deployment.testERC20,
).balanceOf(destination.address)) as bigint;
console.log(
  `unshielded ${formatAmount(amount, decimals)} from ${values.from} to EOA ${values['to-eoa']} (${destination.address}); fee ${formatAmount(fee, decimals)}; ${values.from} private ${formatAmount(fromBefore, decimals)} -> ${formatAmount(fromAfter, decimals)}; EOA public ${formatAmount(destinationBefore, decimals)} -> ${formatAmount(destinationAfter, decimals)}`,
);
await stopEngine();
process.exit(0);
