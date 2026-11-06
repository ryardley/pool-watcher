export interface KeyService {
  decrypt(keyCiphertext: string): Promise<Uint8Array>;
}

/**
 * Dev/local implementation: `keyCiphertext` is `base64(viewingKeyHex)`, where
 * `viewingKeyHex` is the shareable viewing key. No real crypto, same wire shape.
 */
export class LocalKeyService implements KeyService {
  async decrypt(keyCiphertext: string): Promise<Uint8Array> {
    const hex = Buffer.from(keyCiphertext, 'base64').toString('utf8');
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
    default:
      throw new Error(`unknown KEY_SERVICE: ${name}`);
  }
}
