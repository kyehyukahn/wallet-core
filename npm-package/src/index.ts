/**
 * @kyehyukahn/wallet-core — chain-agnostic core
 *
 * Mnemonic management, HD derivation, and CoinType enum. Per-chain signing
 * lives in the subpath exports:
 *
 *   import { signTransaction } from '@kyehyukahn/wallet-core/evm';
 *   import { signTransfer }   from '@kyehyukahn/wallet-core/solana';
 */

export { CoinType } from './CoinType';
export { HDWallet } from './HDWallet';
export type { HexString } from './types';
