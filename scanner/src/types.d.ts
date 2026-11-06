declare module '@railgun-community/circomlibjs' {
  export const babyjub: {
    packPoint(point: [bigint, bigint] | Uint8Array): Buffer;
    unpackPoint(packed: Uint8Array): [bigint, bigint];
  };
  export function poseidon(inputs: bigint[]): bigint;
}

declare module 'msgpack-lite' {
  interface Msgpack {
    decode(buffer: Buffer | Uint8Array): unknown;
    encode(value: unknown): Buffer;
  }
  const msgpack: Msgpack;
  export default msgpack;
}
