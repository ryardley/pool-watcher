import fs from 'node:fs';
import net from 'node:net';
import type { KeyService } from './key-service.js';
import { deriveWallet, type DerivedWallet } from './keys.js';
import { scanBlocks } from './decrypt.js';
import { RequestSchema, type Request, type Response } from './wire.js';

export type ScannerOptions = {
  socketPath: string;
  keyService: KeyService;
};

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function createServer(options: ScannerOptions): net.Server {
  const { socketPath, keyService } = options;
  let wallet: DerivedWallet | null = null;
  let inited = false;

  async function handleRequest(req: Request): Promise<Response> {
    switch (req.op) {
      case 'ping':
        return { op: 'pong' };
      case 'init': {
        if (inited) {
          return { op: 'error', message: 'already initialized' };
        }
        try {
          const bytes = await keyService.decrypt(req.keyCiphertext);
          wallet = await deriveWallet(bytes);
          inited = true;
          return { op: 'ready', walletAddress: wallet.address };
        } catch (err) {
          return {
            op: 'error',
            message: `invalid viewing key: ${errorMessage(err)}`,
          };
        }
      }
      case 'scan': {
        if (!inited || !wallet) {
          return { op: 'error', message: 'not initialized' };
        }
        const events = await scanBlocks(
          {
            addressData: wallet.addressData,
            viewingPrivateKey: wallet.viewingPrivateKey,
            walletAddress: wallet.address,
          },
          req.blocks,
        );
        return { op: 'events', events };
      }
      case 'pubkey': {
        try {
          return { op: 'pubkey', publicKey: keyService.publicKey() };
        } catch (err) {
          return { op: 'error', message: `pubkey unavailable: ${errorMessage(err)}` };
        }
      }
      default:
        return { op: 'error', message: 'unknown op' };
    }
  }

  // Single-flight: serve one client connection at a time. Additional
  // connections wait in a queue until the active client disconnects.
  const pending: net.Socket[] = [];
  let active: net.Socket | null = null;

  function serve(socket: net.Socket): void {
    if (socket.destroyed) {
      const next = pending.shift();
      if (next) serve(next);
      return;
    }
    active = socket;
    socket.setEncoding('utf8');
    socket.on('error', () => {
      socket.destroy();
    });
    socket.on('close', () => {
      active = null;
      const next = pending.shift();
      if (next) serve(next);
    });

    let buffer = '';
    const lineQueue: string[] = [];
    let processing = false;

    async function processNext(): Promise<void> {
      if (processing) return;
      const line = lineQueue.shift();
      if (line === undefined) return;
      processing = true;
      try {
        let resp: Response;
        try {
          const parsed = RequestSchema.safeParse(JSON.parse(line));
          if (!parsed.success) {
            resp = {
              op: 'error',
              message: `invalid request: ${parsed.error.issues
                .map((issue) => `${issue.path.join('.')}: ${issue.message}`)
                .join('; ')}`,
            };
          } else {
            resp = await handleRequest(parsed.data);
          }
        } catch (err) {
          resp = {
            op: 'error',
            message: `invalid request: ${errorMessage(err)}`,
          };
        }
        socket.write(`${JSON.stringify(resp)}\n`);
      } finally {
        processing = false;
        void processNext();
      }
    }

    socket.on('data', (chunk) => {
      buffer += chunk;
      const parts = buffer.split('\n');
      buffer = parts.pop() ?? '';
      for (const part of parts) {
        if (part.trim().length === 0) continue;
        lineQueue.push(part);
        void processNext();
      }
    });
  }

  const server = net.createServer((socket) => {
    if (active) {
      pending.push(socket);
      socket.on('close', () => {
        const index = pending.indexOf(socket);
        if (index >= 0) pending.splice(index, 1);
      });
      return;
    }
    serve(socket);
  });

  // Remove a stale socket file left by a previous run before binding.
  try {
    fs.unlinkSync(socketPath);
  } catch {
    // No stale socket.
  }
  server.listen(socketPath);
  return server;
}
