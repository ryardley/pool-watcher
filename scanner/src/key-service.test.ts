import { test } from 'node:test';
import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { x25519 } from '@noble/curves/ed25519';
import { hkdf } from '@noble/hashes/hkdf';
import { sha256 } from '@noble/hashes/sha256';
import { xchacha20poly1305 } from '@noble/ciphers/chacha';
import { LocalKeyService, SealedBoxKeyService } from './key-service.js';

const HKDF_INFO = new TextEncoder().encode('railgun-scanner-vk');
const NONCE_LEN = 24;
const PUB_LEN = 32;

// Mirrors `app/web/seal.ts`: the browser-side seal the scanner must open.
function seal(plaintext: string, recipientPubB64: string): string {
  const recipientPub = Buffer.from(recipientPubB64, 'base64');
  const ephPriv = x25519.utils.randomPrivateKey();
  const ephPub = x25519.getPublicKey(ephPriv);
  const shared = x25519.getSharedSecret(ephPriv, recipientPub);
  const key = hkdf(sha256, shared, new Uint8Array(0), HKDF_INFO, 32);
  const nonce = randomBytes(NONCE_LEN);
  const ciphertext = xchacha20poly1305(key, nonce).encrypt(
    new TextEncoder().encode(plaintext),
  );
  const envelope = new Uint8Array(PUB_LEN + NONCE_LEN + ciphertext.length);
  envelope.set(ephPub, 0);
  envelope.set(nonce, PUB_LEN);
  envelope.set(ciphertext, PUB_LEN + NONCE_LEN);
  return Buffer.from(envelope).toString('base64');
}

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
