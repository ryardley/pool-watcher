import { ByteLength, ByteUtils, getPublicViewingKey } from '@railgun-community/engine';
import { babyjub, poseidon } from '@railgun-community/circomlibjs';
import { bech32m } from '@scure/base';
import msgpack from 'msgpack-lite';
import { z } from 'zod';

const ShareableViewingKeySchema = z.object({
  vpriv: z.string(),
  spub: z.string(),
});

export type AddressData = {
  masterPublicKey: bigint;
  viewingPublicKey: Uint8Array;
};

export type DerivedWallet = {
  addressData: AddressData;
  viewingPrivateKey: Uint8Array;
  address: string;
};

function xorNetworkID(networkID: string): string {
  const a = Buffer.from(networkID, 'hex');
  const b = Buffer.from('railgun', 'utf8');
  const out = Buffer.alloc(a.length);
  for (let i = 0; i < a.length; i++) out[i] = a[i] ^ b[i];
  return out.toString('hex');
}

export function encodeAddress(addressData: AddressData): string {
  const mpk = ByteUtils.nToHex(addressData.masterPublicKey, ByteLength.UINT_256, false);
  const vpk = ByteUtils.formatToByteLength(
    addressData.viewingPublicKey,
    ByteLength.UINT_256,
  );
  const networkID = 'ffffffffffffffff';
  const addressString = `01${mpk}${xorNetworkID(networkID)}${vpk}`;
  return bech32m.encode(
    '0zk',
    bech32m.toWords(Buffer.from(addressString, 'hex')),
    127,
  );
}

export function decodeShareableViewingKey(
  bytes: Uint8Array,
): { viewingPrivateKey: Uint8Array; spendingPublicKey: [bigint, bigint] } {
  const { vpriv, spub } = ShareableViewingKeySchema.parse(
    msgpack.decode(Buffer.from(bytes)),
  );
  const viewingPrivateKey = ByteUtils.hexStringToBytes(vpriv);
  const spendingPublicKey = babyjub.unpackPoint(Buffer.from(spub, 'hex'));
  return { viewingPrivateKey, spendingPublicKey };
}

export async function deriveWallet(bytes: Uint8Array): Promise<DerivedWallet> {
  const { viewingPrivateKey, spendingPublicKey } = decodeShareableViewingKey(bytes);
  const viewingPublicKey = await getPublicViewingKey(viewingPrivateKey);
  const nullifyingKey = poseidon([
    BigInt(`0x${Buffer.from(viewingPrivateKey).toString('hex')}`),
  ]);
  const masterPublicKey = poseidon([...spendingPublicKey, nullifyingKey]);
  const addressData = { masterPublicKey, viewingPublicKey };
  return {
    addressData,
    viewingPrivateKey,
    address: encodeAddress(addressData),
  };
}
