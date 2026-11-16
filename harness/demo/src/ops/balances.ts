import { parseArgs } from 'node:util';
import {
  eoa,
  formatAmount,
  loadWallet,
  privateBalance,
  readDeployment,
  startEngine,
  stopEngine,
  syncWallets,
  tokenContract,
  tokenDecimals,
} from '../lib.js';

const { values } = parseArgs({
  options: {
    wallets: { type: 'string', default: 'A,B' },
    eoas: { type: 'string', default: '' },
  },
});

const walletNames = values.wallets
  .split(',')
  .map((name) => name.trim())
  .filter(Boolean);
const eoaIndexes = values.eoas
  .split(',')
  .map((index) => index.trim())
  .filter(Boolean)
  .map(Number);

await startEngine();
const deployment = readDeployment();
const decimals = await tokenDecimals(deployment.testERC20);
const records: [string, Awaited<ReturnType<typeof loadWallet>>][] = [];
for (const name of walletNames) {
  records.push([name, await loadWallet(name)]);
}
if (records.length > 0) {
  await syncWallets(records.map(([, record]) => record.id));
}

for (const [name, record] of records) {
  const balance = await privateBalance(record.id, deployment.testERC20);
  console.log(`${name} private: ${formatAmount(balance, decimals)}`);
}
for (const index of eoaIndexes) {
  const wallet = eoa(index);
  const balance = (await tokenContract(deployment.testERC20).balanceOf(
    wallet.address,
  )) as bigint;
  console.log(
    `EOA ${index} (${wallet.address}) public: ${formatAmount(balance, decimals)}`,
  );
}
await stopEngine();
process.exit(0);
