import { createHash } from 'node:crypto';
import { CURVE, Point, utils } from '@noble/ed25519';

function adjustBytes25519(bytes: Uint8Array, endian: 'be' | 'le'): Uint8Array {
  const adjusted = new Uint8Array(bytes);
  if (endian === 'be') {
    adjusted[31] &= 0b11111000;
    adjusted[0] &= 0b01111111;
    adjusted[0] |= 0b01000000;
  } else {
    adjusted[0] &= 0b11111000;
    adjusted[31] &= 0b01111111;
    adjusted[31] |= 0b01000000;
  }
  return adjusted;
}

async function getPrivateScalarFromPrivateKey(privateKey: Uint8Array): Promise<bigint> {
  if (privateKey.length !== 32) throw new Error('Expected 32 bytes');
  const hash = await utils.sha512(privateKey);
  const head = adjustBytes25519(hash.slice(0, 32), 'le');
  const scalar =
    BigInt(`0x${Buffer.from(head.reverse()).toString('hex')}`) % CURVE.l;
  return scalar > 0n ? scalar : CURVE.l;
}

export async function getSharedSymmetricKey(
  privateKeyPairA: Uint8Array,
  blindedPublicKeyPairB: Uint8Array,
): Promise<Uint8Array | undefined> {
  try {
    const scalar = await getPrivateScalarFromPrivateKey(privateKeyPairA);
    const keyPreimage = Point.fromHex(
      Buffer.from(blindedPublicKeyPairB).toString('hex'),
    )
      .multiply(scalar)
      .toRawBytes();
    return createHash('sha256').update(keyPreimage).digest();
  } catch {
    return undefined;
  }
}
