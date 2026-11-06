import { exportKeys, startEngine, stopEngine } from '../lib.js';

await startEngine();
await exportKeys();
console.log('wrote out/keys.json');
await stopEngine();
process.exit(0);
