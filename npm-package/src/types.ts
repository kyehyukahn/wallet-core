/**
 * Chain-agnostic primitive types shared by all subpaths.
 *
 * Chain-specific types (EVM Address, Solana base58 address, etc.) live in
 * their respective subpaths (evm/types.ts, solana/types.ts).
 */

export type HexString = `0x${string}`;
