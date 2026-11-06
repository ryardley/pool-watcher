import {
  ByteLength,
  ByteUtils,
  ShieldNote,
  TransactNote,
  TXIDVersion,
  getNoteHash,
  serializeTokenData,
} from '@railgun-community/engine';
import type { AddressData } from './keys.js';
import { encodeAddress } from './keys.js';
import { getSharedSymmetricKey } from './shared-key.js';
import { tokenDataGetter } from './token-data-getter.js';
import type { ParsedRailgunLog } from './abi.js';
import { parseLog } from './abi.js';
import type { BlockLogs, Event } from './wire.js';

const TXID_VERSION = TXIDVersion.V2_PoseidonMerkle;

type ShieldLog = Extract<ParsedRailgunLog, { name: 'Shield' }>;
type TransactLog = Extract<ParsedRailgunLog, { name: 'Transact' }>;
type UnshieldLog = Extract<ParsedRailgunLog, { name: 'Unshield' }>;

export type ScanContext = {
  chainId: number;
  addressData: AddressData;
  viewingPrivateKey: Uint8Array;
  walletAddress: string;
};

export type ScanWallet = {
  addressData: AddressData;
  viewingPrivateKey: Uint8Array;
  walletAddress: string;
};

type EventFields = Pick<
  Event,
  | 'hash'
  | 'token'
  | 'value'
  | 'outputType'
  | 'direction'
  | 'sender'
  | 'receiver'
  | 'memo'
>;

function kindOf(name: ParsedRailgunLog['name']): Event['kind'] {
  switch (name) {
    case 'Shield':
      return 'shield';
    case 'Transact':
      return 'transact';
    case 'Unshield':
      return 'unshield';
  }
}

function logLinkage(
  log: ParsedRailgunLog,
  ctx: ScanContext,
  fields: EventFields,
): Event {
  return {
    chainId: ctx.chainId,
    walletAddress: ctx.walletAddress,
    address: log.address,
    blockNumber: log.blockNumber,
    txHash: log.txHash,
    logIndex: log.logIndex,
    kind: kindOf(log.name),
    ...fields,
  };
}

async function shieldEvents(ctx: ScanContext, log: ShieldLog): Promise<Event[]> {
  const out: Event[] = [];
  for (let i = 0; i < log.args.commitments.length; i++) {
    const commitment = log.args.commitments[i];
    const ciphertext = log.args.shieldCiphertext[i];
    if (!ciphertext) continue;
    const sharedKey = await getSharedSymmetricKey(
      ctx.viewingPrivateKey,
      ByteUtils.hexToBytes(ciphertext.shieldKey),
    );
    if (!sharedKey) continue;
    try {
      ShieldNote.decryptRandom(ciphertext.encryptedBundle, sharedKey);
      const tokenData = serializeTokenData(
        commitment.token.tokenAddress,
        commitment.token.tokenType,
        commitment.token.tokenSubID.toString(),
      );
      const hash = getNoteHash(commitment.npk, tokenData, commitment.value);
      out.push(
        logLinkage(log, ctx, {
          hash: ByteUtils.nToHex(hash, ByteLength.UINT_256),
          token: tokenData.tokenAddress,
          value: commitment.value.toString(),
          outputType: null,
          direction: 'in',
          sender: null,
          receiver: ctx.walletAddress,
          memo: null,
        }),
      );
    } catch {
      // Shield bundle not addressed to this wallet.
    }
  }
  return out;
}

type CommitmentCiphertext = TransactLog['args']['ciphertext'][number];

function formatCommitmentCiphertext(ct: CommitmentCiphertext) {
  const ivTag = ByteUtils.formatToByteLength(ct.ciphertext[0], ByteLength.UINT_256);
  return {
    ciphertext: {
      iv: ivTag.substring(0, 32),
      tag: ivTag.substring(32),
      data: ct.ciphertext
        .slice(1)
        .map((el) => ByteUtils.formatToByteLength(el, ByteLength.UINT_256)),
    },
    blindedSenderViewingKey: ByteUtils.formatToByteLength(
      ct.blindedSenderViewingKey,
      ByteLength.UINT_256,
    ),
    blindedReceiverViewingKey: ByteUtils.formatToByteLength(
      ct.blindedReceiverViewingKey,
      ByteLength.UINT_256,
    ),
    annotationData: ct.annotationData,
    memo: ct.memo,
  };
}

async function tryDecryptTransact(
  ctx: ScanContext,
  log: TransactLog,
  cc: ReturnType<typeof formatCommitmentCiphertext>,
  blindedSender: Uint8Array,
  blindedReceiver: Uint8Array,
  sharedKey: Uint8Array,
  isSentNote: boolean,
): Promise<Event | undefined> {
  try {
    const note = await TransactNote.decrypt(
      TXID_VERSION,
      { type: 0, id: ctx.chainId },
      ctx.addressData,
      cc.ciphertext,
      sharedKey,
      cc.memo,
      cc.annotationData,
      ctx.viewingPrivateKey,
      blindedReceiver,
      blindedSender,
      isSentNote,
      false,
      tokenDataGetter,
      log.blockNumber,
      undefined,
    );
    return logLinkage(log, ctx, {
      hash: ByteUtils.nToHex(note.hash, ByteLength.UINT_256),
      token: note.tokenData.tokenAddress,
      value: note.value.toString(),
      outputType: note.outputType ?? null,
      direction: isSentNote ? 'out' : 'in',
      sender: note.senderAddressData
        ? encodeAddress(note.senderAddressData)
        : null,
      receiver: encodeAddress(note.receiverAddressData),
      memo: note.memoText ?? null,
    });
  } catch {
    return undefined;
  }
}

async function transactEvents(
  ctx: ScanContext,
  log: TransactLog,
): Promise<Event[]> {
  const out: Event[] = [];
  for (const ct of log.args.ciphertext) {
    const cc = formatCommitmentCiphertext(ct);
    const blindedSender = ByteUtils.hexStringToBytes(cc.blindedSenderViewingKey);
    const blindedReceiver = ByteUtils.hexStringToBytes(cc.blindedReceiverViewingKey);
    const [sharedKeyReceiver, sharedKeySender] = await Promise.all([
      getSharedSymmetricKey(ctx.viewingPrivateKey, blindedSender),
      getSharedSymmetricKey(ctx.viewingPrivateKey, blindedReceiver),
    ]);
    if (sharedKeyReceiver) {
      const event = await tryDecryptTransact(
        ctx, log, cc, blindedSender, blindedReceiver, sharedKeyReceiver, false,
      );
      if (event) out.push(event);
    }
    if (sharedKeySender) {
      const event = await tryDecryptTransact(
        ctx, log, cc, blindedSender, blindedReceiver, sharedKeySender, true,
      );
      if (event) out.push(event);
    }
  }
  return out;
}

function unshieldEvents(ctx: ScanContext, log: UnshieldLog): Event[] {
  // The contract emits the net amount (after fee); the note value that leaves
  // the private balance is the gross amount.
  const value = log.args.amount + log.args.fee;
  return [
    logLinkage(log, ctx, {
      hash: null,
      token: log.args.token.tokenAddress,
      value: value.toString(),
      outputType: null,
      direction: 'out',
      sender: ctx.walletAddress,
      receiver: log.args.to,
      memo: null,
    }),
  ];
}

export async function scanBlocks(
  wallet: ScanWallet,
  blocks: BlockLogs[],
): Promise<Event[]> {
  // Unshield events are public and carry no sender, so attribute them to the
  // wallet that can decrypt a note in the same transaction (the change note).
  // Process in two passes to keep the emitted events in log order.
  type Pending =
    | { kind: 'events'; events: Event[] }
    | { kind: 'unshield'; ctx: ScanContext; log: UnshieldLog };

  const pending: Pending[] = [];
  const involvedTx = new Set<string>();

  for (const block of blocks) {
    const ctx: ScanContext = {
      chainId: block.chainId,
      addressData: wallet.addressData,
      viewingPrivateKey: wallet.viewingPrivateKey,
      walletAddress: wallet.walletAddress,
    };
    for (const log of block.logs) {
      const parsed = parseLog(log);
      if (!parsed) continue;
      if (parsed.name === 'Shield') {
        pending.push({ kind: 'events', events: await shieldEvents(ctx, parsed) });
      } else if (parsed.name === 'Transact') {
        const events = await transactEvents(ctx, parsed);
        if (events.length > 0) involvedTx.add(parsed.txHash);
        pending.push({ kind: 'events', events });
      } else {
        pending.push({ kind: 'unshield', ctx, log: parsed });
      }
    }
  }

  const events: Event[] = [];
  for (const item of pending) {
    if (item.kind === 'events') {
      events.push(...item.events);
    } else if (involvedTx.has(item.log.txHash)) {
      events.push(...unshieldEvents(item.ctx, item.log));
    }
  }
  return events;
}
