import { z } from 'zod';

export const RawLogSchema = z.object({
  address: z.string(),
  topics: z.array(z.string()),
  data: z.string(),
  blockNumber: z.union([z.string(), z.number()]).optional(),
  logIndex: z.union([z.string(), z.number()]).optional(),
  transactionHash: z.string().optional(),
});
export type RawLog = z.infer<typeof RawLogSchema>;

export const BlockLogsSchema = z.object({
  chainId: z.number(),
  logs: z.array(RawLogSchema),
});
export type BlockLogs = z.infer<typeof BlockLogsSchema>;

export const RequestSchema = z.discriminatedUnion('op', [
  z.object({ op: z.literal('init'), keyCiphertext: z.string().min(1) }),
  z.object({ op: z.literal('scan'), blocks: z.array(BlockLogsSchema) }),
  z.object({ op: z.literal('ping') }),
]);
export type Request = z.infer<typeof RequestSchema>;

export const EventSchema = z.object({
  chainId: z.number(),
  walletAddress: z.string(),
  address: z.string(),
  blockNumber: z.number(),
  txHash: z.string(),
  logIndex: z.number(),
  kind: z.enum(['shield', 'transact', 'unshield']),
  hash: z.string().nullable(),
  token: z.string(),
  value: z.string(),
  outputType: z.number().nullable(),
  direction: z.enum(['in', 'out']),
  sender: z.string().nullable(),
  receiver: z.string().nullable(),
  memo: z.string().nullable(),
});
export type Event = z.infer<typeof EventSchema>;

export const ReadyResponseSchema = z.object({
  op: z.literal('ready'),
  walletAddress: z.string(),
});
export const EventsResponseSchema = z.object({
  op: z.literal('events'),
  events: z.array(EventSchema),
});
export const PongResponseSchema = z.object({ op: z.literal('pong') });
export const ErrorResponseSchema = z.object({
  op: z.literal('error'),
  message: z.string(),
});
export const ResponseSchema = z.discriminatedUnion('op', [
  ReadyResponseSchema,
  EventsResponseSchema,
  PongResponseSchema,
  ErrorResponseSchema,
]);
export type Response = z.infer<typeof ResponseSchema>;
