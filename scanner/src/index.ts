import fs from 'node:fs';
import { createKeyService } from './key-service.js';
import { createServer } from './server.js';

const socketPath = process.env.SCANNER_SOCKET;
if (!socketPath) {
  console.error('SCANNER_SOCKET must be set to a Unix socket path');
  process.exit(1);
}
const path: string = socketPath;

const keyService = createKeyService(process.env.KEY_SERVICE ?? 'local');
const server = createServer({ socketPath: path, keyService });

function shutdown(): void {
  try {
    fs.unlinkSync(path);
  } catch {
    // Socket file already gone.
  }
  server.close();
  process.exit(0);
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);
process.on('SIGHUP', shutdown);
