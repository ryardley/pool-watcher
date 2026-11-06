import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tokenDataGetter } from './token-data-getter.js';

test('tokenDataGetter shim reconstructs ERC-20 token data from a zero-padded hash', async () => {
  const tokenAddress = '0x6C2d83262fF84cBaDb3e416D527403135D757892';
  const tokenHash = `0x${'0'.repeat(24)}${tokenAddress.slice(2).toLowerCase()}`;
  const tokenData = await tokenDataGetter.getTokenDataFromHash(
    'V2_PoseidonMerkle',
    { type: 0, id: 31337 },
    tokenHash,
  );
  assert.equal(tokenData.tokenAddress.toLowerCase(), tokenAddress.toLowerCase());
  assert.equal(tokenData.tokenType, 0);
});
