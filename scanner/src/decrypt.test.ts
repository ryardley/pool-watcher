import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { deriveWallet } from './keys.js';
import { scanBlocks } from './decrypt.js';
import { LocalKeyService } from './key-service.js';
import type { BlockLogs, Event } from './wire.js';

const ROOT = path.resolve(import.meta.dirname, '../..');
const keys = JSON.parse(
  fs.readFileSync(path.join(ROOT, 'out/keys.json'), 'utf8'),
) as Record<string, { shareableViewingKey: string; railgunAddress: string }>;
const fixture = JSON.parse(
  fs.readFileSync(path.join(ROOT, 'out/logs.json'), 'utf8'),
) as { blocks: BlockLogs[] };

const EOA2 = '0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC';

async function scanFor(name: string): Promise<Event[]> {
  const ciphertext = Buffer.from(
    keys[name].shareableViewingKey,
    'utf8',
  ).toString('base64');
  const bytes = await new LocalKeyService().decrypt(ciphertext);
  const wallet = await deriveWallet(bytes);
  return scanBlocks(
    {
      addressData: wallet.addressData,
      viewingPrivateKey: wallet.viewingPrivateKey,
      walletAddress: wallet.address,
    },
    fixture.blocks,
  );
}

function balance(events: Event[]): bigint {
  let total = 0n;
  for (const event of events) {
    const value = BigInt(event.value);
    total += event.direction === 'in' ? value : -value;
  }
  return total;
}

function transfers(events: Event[], direction: 'in' | 'out'): Event[] {
  return events.filter(
    (e) => e.kind === 'transact' && e.direction === direction && e.memo !== null,
  );
}

test('A: shield, five transfers out, change notes, no unshield', async () => {
  const events = await scanFor('A');

  const shield = events.find((e) => e.kind === 'shield');
  assert.ok(shield);
  assert.equal(shield.direction, 'in');
  assert.equal(shield.value, '997500000000000000000');
  assert.equal(shield.receiver, keys.A.railgunAddress);
  assert.equal(shield.sender, null);

  const outs = transfers(events, 'out');
  assert.deepEqual(
    outs.map((e) => e.value),
    [
      '10000000000000000000',
      '25000000000000000000',
      '40000000000000000000',
      '60000000000000000000',
      '100000000000000000000',
    ],
  );
  assert.deepEqual(
    outs.map((e) => e.memo),
    ['transfer #0', 'transfer #1', 'transfer #2', 'transfer #3', 'transfer #4'],
  );
  for (const event of outs) {
    assert.equal(event.sender, keys.A.railgunAddress);
    assert.equal(event.receiver, keys.B.railgunAddress);
    assert.equal(event.outputType, 0);
  }

  assert.equal(events.filter((e) => e.kind === 'unshield').length, 0);
  assert.equal(balance(events), 762_500_000_000_000_000_000n);
});

test('B: five transfers in with sender visibility, unshield, change note', async () => {
  const events = await scanFor('B');

  const ins = transfers(events, 'in');
  assert.deepEqual(
    ins.map((e) => e.value),
    [
      '10000000000000000000',
      '25000000000000000000',
      '40000000000000000000',
      '60000000000000000000',
      '100000000000000000000',
    ],
  );
  assert.deepEqual(
    ins.map((e) => e.memo),
    ['transfer #0', 'transfer #1', 'transfer #2', 'transfer #3', 'transfer #4'],
  );
  // Sender is visible only for the transfers that opted in (0, 2, 4).
  assert.deepEqual(
    ins.map((e) => e.sender),
    [
      keys.A.railgunAddress,
      null,
      keys.A.railgunAddress,
      null,
      keys.A.railgunAddress,
    ],
  );

  const unshield = events.find((e) => e.kind === 'unshield');
  assert.ok(unshield);
  assert.equal(unshield.direction, 'out');
  assert.equal(unshield.value, '50000000000000000000');
  assert.equal(unshield.receiver, EOA2);
  assert.equal(unshield.sender, keys.B.railgunAddress);

  assert.equal(balance(events), 185_000_000_000_000_000_000n);
});

test('event shaping echoes chain and log linkage', async () => {
  const events = await scanFor('A');
  for (const event of events) {
    assert.equal(event.chainId, 31337);
    assert.equal(event.walletAddress, keys.A.railgunAddress);
    assert.equal(event.address, '0x99dBE4AEa58E518C50a1c04aE9b48C9F6354612f');
    assert.ok(event.blockNumber >= 214);
    assert.match(event.txHash, /^0x[0-9a-f]{64}$/);
    assert.ok(event.logIndex >= 0);
    assert.ok(event.direction === 'in' || event.direction === 'out');
  }
});

test('change notes decrypt in both directions and cancel out', async () => {
  const events = await scanFor('A');
  const changes = events.filter(
    (e) => e.kind === 'transact' && e.outputType === 2,
  );
  assert.equal(changes.length, 5);
  for (const change of changes) {
    const pair = events.find(
      (e) =>
        e.kind === 'transact' &&
        e.hash === change.hash &&
        e.direction !== change.direction,
    );
    assert.ok(pair, 'change note has a matching opposite-direction event');
    assert.equal(pair.value, change.value);
  }
});
