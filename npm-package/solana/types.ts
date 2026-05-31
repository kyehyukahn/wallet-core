/**
 * Solana-specific domain types for @kyehyukahn/wallet-core/solana.
 *
 * v0.2.0 supports the SOL transfer instruction only. Token transfers,
 * stake / vote / nonce / proposal etc. are out of scope and can be added
 * by extending SolanaSigning.{swift,kt} + new TS helpers without touching
 * the chain-agnostic core.
 */

/** Solana base58-encoded ed25519 public key (32 bytes). */
export type SolanaAddress = string;

/** 32-byte base58 blockhash, fetched from a Solana RPC via getLatestBlockhash. */
export type Blockhash = string;

export interface SolanaTransfer {
  /** Recipient base58 address. */
  recipient: SolanaAddress;
  /** Amount in lamports, decimal string (uint64). 1 SOL = 1e9 lamports. */
  lamports: string;
  /** Recent blockhash; transactions expire ~150 blocks after the slot it was sampled. */
  recentBlockhash: Blockhash;
  /** Optional UTF-8 memo. */
  memo?: string;
}
