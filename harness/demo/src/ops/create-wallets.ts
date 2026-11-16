import fs from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';
import {
  ROOT,
  createWallet,
  readWallets,
  startEngine,
  stopEngine,
  writeWallets,
} from '../lib.js';

const { values } = parseArgs({
  options: { names: { type: 'string', default: 'A,B' } },
});

const names = values.names
  .split(',')
  .map((name) => name.trim())
  .filter(Boolean);
const mnemonicFiles = ['.wallet1', '.wallet2'];
if (names.length > mnemonicFiles.length) {
  throw new Error(`at most ${mnemonicFiles.length} wallets are supported`);
}

await startEngine();
const wallets = readWallets();
for (const [index, name] of names.entries()) {
  if (wallets[name]) {
    console.log(`skip: wallet ${name} already exists`);
    continue;
  }
  const mnemonic = fs
    .readFileSync(path.join(ROOT, mnemonicFiles[index]), 'utf8')
    .trim();
  wallets[name] = await createWallet(name, mnemonic);
  console.log(`created: wallet ${name} ${wallets[name].railgunAddress}`);
}
writeWallets(wallets);
await stopEngine();
process.exit(0);
