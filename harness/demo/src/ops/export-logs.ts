import fs from 'node:fs';
import path from 'node:path';
import { Interface } from 'ethers';
import { OUT_DIR, ROOT, provider, readDeployment } from '../lib.js';

const deployment = readDeployment();

const artifact = JSON.parse(
  fs.readFileSync(
    path.join(
      ROOT,
      'vendor/contract/artifacts/contracts/logic/RailgunSmartWallet.sol/RailgunSmartWallet.json',
    ),
    'utf8',
  ),
) as { abi: unknown[] };
const iface = new Interface(artifact.abi);
const topics = ['Shield', 'Transact', 'Unshield'].map(
  (name) => iface.getEvent(name)!.topicHash,
);

const logs = await provider.getLogs({
  address: deployment.proxy,
  fromBlock: deployment.deploymentBlock,
  toBlock: 'latest',
});

const byBlock = new Map<number, unknown[]>();
for (const log of logs) {
  if (!topics.includes(log.topics[0])) continue;
  const blockNumber = Number(log.blockNumber);
  if (!byBlock.has(blockNumber)) byBlock.set(blockNumber, []);
  byBlock.get(blockNumber)!.push({
    address: log.address,
    topics: log.topics,
    data: log.data,
    blockNumber: log.blockNumber,
    logIndex: log.index,
    transactionHash: log.transactionHash,
  });
}

const blocks = [...byBlock.entries()]
  .sort((a, b) => a[0] - b[0])
  .map(([, blockLogs]) => ({ chainId: 31337, logs: blockLogs }));

const fixture = {
  chainId: 31337,
  proxy: deployment.proxy,
  deploymentBlock: deployment.deploymentBlock,
  blocks,
};

fs.mkdirSync(OUT_DIR, { recursive: true });
fs.writeFileSync(
  path.join(OUT_DIR, 'logs.json'),
  `${JSON.stringify(fixture, null, 2)}\n`,
);
console.log(
  `wrote out/logs.json (${blocks.length} blocks, ${blocks.reduce((n, b) => n + b.logs.length, 0)} logs)`,
);
