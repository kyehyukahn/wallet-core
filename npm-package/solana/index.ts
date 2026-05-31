import { requireNativeModule } from 'expo-modules-core';
import { CoinType } from '../src/CoinType';
import type { SolanaTransfer, SolanaAddress } from './types';

export type { SolanaTransfer, SolanaAddress, Blockhash } from './types';

/** Standard BIP-44 Solana account 0 derivation path (all-hardened). */
export const SOLANA_DEFAULT_PATH = "m/44'/501'/0'/0'";

const UINT64_MAX = (1n << 64n) - 1n;

interface SolanaTxNative {
  recipient: string;
  lamports: string;
  recentBlockhash: string;
  memo?: string;
}

interface SolanaBindings {
  solanaSignTransfer(tx: SolanaTxNative, mnemonic: string, path: string): Promise<string>;
  deriveAddress(mnemonic: string, coin: number, path: string): Promise<string>;
}

const Native = requireNativeModule('WalletCoreModule') as unknown as SolanaBindings;

/**
 * Sign a SOL transfer with the key derived from `mnemonic` at `derivationPath`.
 * Returns the wallet-core SigningOutput.encoded string (a base64/base58 encoded
 * transaction, ready for submission to a Solana RPC).
 *
 * `recentBlockhash` must be a recent (within ~150 slots) blockhash fetched
 * from a Solana RPC. Caller is responsible for refresh / retry.
 */
export function signTransfer(
  tx: SolanaTransfer,
  mnemonic: string,
  derivationPath: string = SOLANA_DEFAULT_PATH,
): Promise<string> {
  let lamports: bigint;
  try {
    lamports = BigInt(tx.lamports);
  } catch {
    throw new Error(`solana.lamports: not a valid integer (${tx.lamports})`);
  }
  if (lamports < 0n) throw new Error('solana.lamports: negative');
  if (lamports > UINT64_MAX) throw new Error('solana.lamports: exceeds uint64');

  const native: SolanaTxNative = {
    recipient: tx.recipient,
    lamports: tx.lamports,
    recentBlockhash: tx.recentBlockhash,
  };
  if (tx.memo !== undefined) native.memo = tx.memo;

  return Native.solanaSignTransfer(native, mnemonic, derivationPath);
}

/** Derive a Solana address at the given derivation path. */
export function deriveAddress(
  mnemonic: string,
  derivationPath: string = SOLANA_DEFAULT_PATH,
): Promise<SolanaAddress> {
  return Native.deriveAddress(mnemonic, CoinType.Solana, derivationPath);
}
