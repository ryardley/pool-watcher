import { getTokenDataERC20 } from '@railgun-community/engine';
import type { TokenDataGetter } from '@railgun-community/engine';

/**
 * Structural adapter for the engine's `TokenDataGetter` class. The class has
 * private fields, so a plain object cannot be assigned to it structurally; this
 * is a type-system boundary, not data parsing. ERC-20 token hashes are the
 * token address zero-padded, so no DB or chain call is needed.
 */
export const tokenDataGetter = {
  getTokenDataFromHash: async (
    _txidVersion: unknown,
    _chain: unknown,
    tokenHash: string,
  ) => getTokenDataERC20(tokenHash),
} as unknown as TokenDataGetter;
