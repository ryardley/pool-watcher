import { Interface } from 'ethers';
import { ABIRailgunSmartWallet } from '@railgun-community/engine';
import { z } from 'zod';
import type { RawLog } from './wire.js';

const iface = new Interface(ABIRailgunSmartWallet);

// ethers decodes nested structs as `Result` tuples (array-like), so the schemas
// describe the tuple layout and transform it into named fields.
const TokenDataSchema = z
  .tuple([z.bigint(), z.string(), z.bigint()])
  .transform(([tokenType, tokenAddress, tokenSubID]) => ({
    tokenType,
    tokenAddress,
    tokenSubID,
  }));

const CommitmentPreimageSchema = z
  .tuple([z.string(), TokenDataSchema, z.bigint()])
  .transform(([npk, token, value]) => ({ npk, token, value }));

const ShieldCiphertextSchema = z
  .tuple([z.tuple([z.string(), z.string(), z.string()]), z.string()])
  .transform(([encryptedBundle, shieldKey]) => ({ encryptedBundle, shieldKey }));

const CommitmentCiphertextSchema = z
  .tuple([
    z.tuple([z.string(), z.string(), z.string(), z.string()]),
    z.string(),
    z.string(),
    z.string(),
    z.string(),
  ])
  .transform(
    ([ciphertext, blindedSenderViewingKey, blindedReceiverViewingKey, annotationData, memo]) => ({
      ciphertext,
      blindedSenderViewingKey,
      blindedReceiverViewingKey,
      annotationData,
      memo,
    }),
  );

const ShieldArgsSchema = z.object({
  treeNumber: z.bigint(),
  startPosition: z.bigint(),
  commitments: z.array(CommitmentPreimageSchema),
  shieldCiphertext: z.array(ShieldCiphertextSchema),
  fees: z.array(z.bigint()),
});

const TransactArgsSchema = z.object({
  treeNumber: z.bigint(),
  startPosition: z.bigint(),
  hash: z.array(z.string()),
  ciphertext: z.array(CommitmentCiphertextSchema),
});

const UnshieldArgsSchema = z.object({
  to: z.string(),
  token: TokenDataSchema,
  amount: z.bigint(),
  fee: z.bigint(),
});

type LogLinkage = {
  address: string;
  blockNumber: number;
  txHash: string;
  logIndex: number;
};

export type ParsedRailgunLog =
  | ({ name: 'Shield'; args: z.infer<typeof ShieldArgsSchema> } & LogLinkage)
  | ({ name: 'Transact'; args: z.infer<typeof TransactArgsSchema> } & LogLinkage)
  | ({ name: 'Unshield'; args: z.infer<typeof UnshieldArgsSchema> } & LogLinkage);

export function parseLog(log: RawLog): ParsedRailgunLog | undefined {
  try {
    const parsed = iface.parseLog({ topics: log.topics, data: log.data });
    if (!parsed) return undefined;
    const linkage: LogLinkage = {
      address: log.address,
      blockNumber: Number(log.blockNumber ?? 0),
      txHash: log.transactionHash ?? '',
      logIndex: Number(log.logIndex ?? 0),
    };
    const rawArgs = parsed.args.toObject();
    if (parsed.name === 'Shield') {
      return { name: 'Shield', args: ShieldArgsSchema.parse(rawArgs), ...linkage };
    }
    if (parsed.name === 'Transact') {
      return {
        name: 'Transact',
        args: TransactArgsSchema.parse(rawArgs),
        ...linkage,
      };
    }
    if (parsed.name === 'Unshield') {
      return {
        name: 'Unshield',
        args: UnshieldArgsSchema.parse(rawArgs),
        ...linkage,
      };
    }
  } catch {
    // Not a Railgun Shield/Transact/Unshield event, or an unexpected shape.
  }
  return undefined;
}
