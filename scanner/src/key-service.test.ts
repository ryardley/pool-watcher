import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LocalKeyService } from './key-service.js';

test('local KeyService decodes base64(viewingKeyHex) to raw key bytes', async () => {
  const hex = '82a57670726976d94063333739366637656631323363393561333838363831613063646636353336353731646539303638646631633735353332376339396635633133336166366264';
  const ciphertext = Buffer.from(hex, 'utf8').toString('base64');
  const bytes = await new LocalKeyService().decrypt(ciphertext);
  assert.equal(Buffer.from(bytes).toString('hex'), hex);
});
