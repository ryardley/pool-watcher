import { x25519 } from '@noble/curves/ed25519';
import { hkdf } from '@noble/hashes/hkdf';
import { sha256 } from '@noble/hashes/sha256';
import { xchacha20poly1305 } from '@noble/ciphers/chacha';

export interface KeyService {
  decrypt(keyCiphertext: string): Promise<Uint8Array>;
  publicKey(): string;
}

const HKDF_INFO = new TextEncoder().encode('railgun-scanner-vk');
const PUB_LEN = 32;
const NONCE_LEN = 24;

/**
 * Dev/local implementation: `keyCiphertext` is `base64(viewingKeyHex)`, where
 * `viewingKeyHex` is the shareable viewing key. No real crypto, same wire shape.
 */
export class LocalKeyService implements KeyService {
  async decrypt(keyCiphertext: string): Promise<Uint8Array> {
    const hex = Buffer.from(keyCiphertext, 'base64').toString('utf8');
    return Buffer.from(hex, 'hex');
  }

  publicKey(): string {
    throw new Error('local KeyService has no public key');
  }
}

/**
 * Demo implementation: the scanner generates an X25519 keypair at boot and
 * opens a sealed box (X25519 + HKDF-SHA256 + XChaCha20-Poly1305) to recover
 * the viewing-key hex string, then hex-decodes it. Envelope (base64):
 * `ephemeralPublicKey(32) || nonce(24) || ciphertext(N+16)`.
 */
export class SealedBoxKeyService implements KeyService {
  private readonly privateKey: Uint8Array;
  private readonly publicKeyB64: string;

  constructor() {
    this.privateKey = x25519.utils.randomPrivateKey();
    this.publicKeyB64 = Buffer.from(x25519.getPublicKey(this.privateKey)).toString(
      'base64',
    );
  }

  publicKey(): string {
    return this.publicKeyB64;
  }

  async decrypt(keyCiphertext: string): Promise<Uint8Array> {
    const envelope = Buffer.from(keyCiphertext, 'base64');
    if (envelope.length < PUB_LEN + NONCE_LEN) {
      throw new Error('sealed envelope too short');
    }
    const ephPub = envelope.subarray(0, PUB_LEN);
    const nonce = envelope.subarray(PUB_LEN, PUB_LEN + NONCE_LEN);
    const ciphertext = envelope.subarray(PUB_LEN + NONCE_LEN);
    const shared = x25519.getSharedSecret(this.privateKey, ephPub);
    const key = hkdf(sha256, shared, new Uint8Array(0), HKDF_INFO, 32);
    const plaintext = xchacha20poly1305(key, nonce).decrypt(ciphertext);
    const hex = Buffer.from(plaintext).toString('utf8');
    return Buffer.from(hex, 'hex');
  }
}

/**
 * Select the KeyService implementation. This is a build/deploy property (the
 * Docker build arg bakes `KEY_SERVICE` into the image), never a wire input.
 * The `nitro` implementation is not built yet.
 */
export function createKeyService(name: string): KeyService {
  switch (name) {
    case 'local':
      return new LocalKeyService();
    case 'sealedbox':
      return new SealedBoxKeyService();
    default:
      throw new Error(`unknown KEY_SERVICE: ${name}`);
  }
}
