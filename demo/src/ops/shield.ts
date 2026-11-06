import { parseArgs } from 'node:util';
import {
  gasEstimateForShield,
  populateShield,
} from '@railgun-community/wallet';
import {
  NETWORK,
  TXID_VERSION,
  eoa,
  erc20Recipient,
  formatAmount,
  gasDetails,
  loadWallet,
  privateBalance,
  readDeployment,
  recordStep,
  shieldFeeFromReceipt,
  shieldPrivateKey,
  startEngine,
  stopEngine,
  syncWallets,
  toBaseUnits,
  tokenContract,
  tokenDecimals,
} from '../lib.js';

const { values } = parseArgs({
  options: {
    'from-eoa': { type: 'string', default: '0' },
    to: { type: 'string' },
    amount: { type: 'string' },
  },
});

if (!values.to || !values.amount) {
  throw new Error('--to and --amount are required');
}

await startEngine();
const wallet = eoa(Number(values['from-eoa']));
const target = await loadWallet(values.to);
const deployment = readDeployment();
const amount = await toBaseUnits(deployment.testERC20, values.amount);
const decimals = await tokenDecimals(deployment.testERC20);
const recipient = erc20Recipient(
  deployment.testERC20,
  amount,
  target.railgunAddress,
);

await syncWallets([target.id]);
const privateBefore = await privateBalance(target.id, deployment.testERC20);
const before = await tokenContract(deployment.testERC20).balanceOf(
  wallet.address,
);

const token = tokenContract(deployment.testERC20, wallet);
const allowance = (await token.allowance(
  wallet.address,
  deployment.proxy,
)) as bigint;
if (allowance < amount) {
  const approval = await token.approve(deployment.proxy, amount);
  await approval.wait();
}

const privateKey = await shieldPrivateKey(wallet);
const { gasEstimate } = await gasEstimateForShield(
  TXID_VERSION,
  NETWORK,
  privateKey,
  [recipient],
  [],
  wallet.address,
);
const details = await gasDetails(gasEstimate, true);
const { transaction } = await populateShield(
  TXID_VERSION,
  NETWORK,
  privateKey,
  [recipient],
  [],
  details,
);
const tx = await wallet.sendTransaction(transaction);
const receipt = await tx.wait();
if (!receipt) throw new Error('shield transaction was not mined');
const fee = shieldFeeFromReceipt(receipt);

recordStep({
  kind: 'shield',
  txHash: receipt.hash,
  block: receipt.blockNumber,
  from: wallet.address,
  to: target.railgunAddress,
  amount: amount.toString(),
  fee: fee.toString(),
  memo: null,
});

await syncWallets([target.id]);
const privateAfter = await privateBalance(target.id, deployment.testERC20);
const after = await tokenContract(deployment.testERC20).balanceOf(
  wallet.address,
);
console.log(
  `shielded ${formatAmount(amount, decimals)} to ${values.to}; fee ${formatAmount(fee, decimals)}; ${values.to} private ${formatAmount(privateBefore, decimals)} -> ${formatAmount(privateAfter, decimals)}; EOA balance ${formatAmount(before as bigint, decimals)} -> ${formatAmount(after as bigint, decimals)}`,
);
await stopEngine();
process.exit(0);
