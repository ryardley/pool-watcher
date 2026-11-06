import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { parseLog } from './abi.js';
import type { RawLog } from './wire.js';

const ROOT = path.resolve(import.meta.dirname, '../..');
const fixture = JSON.parse(
  fs.readFileSync(path.join(ROOT, 'out/logs.json'), 'utf8'),
) as { blocks: Array<{ logs: RawLog[] }> };

test('parses Shield/Transact/Unshield logs from the fixture', () => {
  const names = fixture.blocks
    .flatMap((block) => block.logs)
    .map((log) => parseLog(log)?.name)
    .filter(Boolean);
  assert.ok(names.includes('Shield'));
  assert.ok(names.includes('Transact'));
  assert.ok(names.includes('Unshield'));
});

test('ignores logs that are not Railgun events', () => {
  const log: RawLog = {
    address: '0x0000000000000000000000000000000000000000',
    topics: ['0x' + '00'.repeat(32)],
    data: '0x',
    blockNumber: 1,
    logIndex: 0,
    transactionHash: '0x' + '00'.repeat(32),
  };
  assert.equal(parseLog(log), undefined);
});
