import { parseArgs } from 'node:util';
import {
  eoa,
  formatAmount,
  readDeployment,
  toBaseUnits,
  tokenContract,
  tokenDecimals,
} from '../lib.js';

const { values } = parseArgs({
  options: {
    eoa: { type: 'string', default: '0' },
    amount: { type: 'string' },
  },
});

if (!values.amount) throw new Error('--amount is required');
const index = Number(values.eoa);
const deployment = readDeployment();
const amount = await toBaseUnits(deployment.testERC20, values.amount);
const decimals = await tokenDecimals(deployment.testERC20);
const wallet = eoa(index);
const token = tokenContract(deployment.testERC20, wallet);
const balance = (await token.balanceOf(wallet.address)) as bigint;

if (balance >= amount) {
  console.log(
    `skip: EOA ${index} already has ${formatAmount(balance, decimals)} tokens`,
  );
  process.exit(0);
}

const shortfall = amount - balance;
const tx = await token.mint(wallet.address, shortfall);
await tx.wait();
const after = (await token.balanceOf(wallet.address)) as bigint;
console.log(
  `minted ${formatAmount(shortfall, decimals)} to EOA ${index} (${wallet.address}); balance ${formatAmount(after, decimals)}`,
);
