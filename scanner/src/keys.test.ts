import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { deriveWallet, encodeAddress } from './keys.js';
import { LocalKeyService } from './key-service.js';

const ROOT = path.resolve(import.meta.dirname, '../..');
const keys = JSON.parse(
  fs.readFileSync(path.join(ROOT, 'out/keys.json'), 'utf8'),
) as Record<string, { shareableViewingKey: string; railgunAddress: string }>;

async function derive(name: string) {
  const ciphertext = Buffer.from(
    keys[name].shareableViewingKey,
    'utf8',
  ).toString('base64');
  const bytes = await new LocalKeyService().decrypt(ciphertext);
  return deriveWallet(bytes);
}

test('derives the wallet 0zk address from the shareable viewing key', async () => {
  for (const name of ['A', 'B']) {
    const wallet = await derive(name);
    assert.equal(wallet.address, keys[name].railgunAddress);
  }
});

test('encodeAddress is stable for the derived address data', async () => {
  const wallet = await derive('A');
  assert.equal(encodeAddress(wallet.addressData), wallet.address);
});

test('deriveWallet rejects bytes that are not a shareable viewing key', async () => {
  await assert.rejects(() => deriveWallet(new Uint8Array([1, 2, 3, 4])));
});
