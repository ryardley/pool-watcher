import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LocalKeyService, SealedBoxKeyService } from './key-service.js';
// The real browser bundle: a VK sealed by `app/web/seal.ts` must open here.
import { seal } from '../../app/web/seal.ts';

const PUB_LEN = 32;

test('local KeyService decodes base64(viewingKeyHex) to raw key bytes', async () => {
  const hex = '82a57670726976d94063333739366637656631323363393561333838363831613063646636353336353731646539303638646631633735353332376339396635633133336166366264';
  const ciphertext = Buffer.from(hex, 'utf8').toString('base64');
  const bytes = await new LocalKeyService().decrypt(ciphertext);
  assert.equal(Buffer.from(bytes).toString('hex'), hex);
});

test('local KeyService has no public key', () => {
  assert.throws(() => new LocalKeyService().publicKey());
});

test('sealedbox KeyService exposes a base64 X25519 public key', () => {
  const service = new SealedBoxKeyService();
  const pub = Buffer.from(service.publicKey(), 'base64');
  assert.equal(pub.length, PUB_LEN);
});

test('sealedbox KeyService opens a sealed viewing key to the original hex', async () => {
  const service = new SealedBoxKeyService();
  const hex = '82a57670726976d94063333739366637656631323363393561333838363831613063646636353336353731646539303638646631633735353332376339396635633133336166366264';
  const ciphertext = seal(hex, service.publicKey());
  const bytes = await service.decrypt(ciphertext);
  assert.equal(Buffer.from(bytes).toString('hex'), hex);
});

test('sealedbox KeyService rejects a box sealed to a different key', async () => {
  const service = new SealedBoxKeyService();
  const other = new SealedBoxKeyService();
  const hex = '82a57670726976d94063333739366637656631323363393561333838363831613063646636353336353731646539303638646631633735353332376339396635633133336166366264';
  const ciphertext = seal(hex, other.publicKey());
  await assert.rejects(() => service.decrypt(ciphertext));
});
