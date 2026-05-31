import { requireNativeModule } from 'expo-modules-core';

/**
 * Chain-agnostic native bindings exposed by WalletCoreModule.
 *
 * Per-chain subpaths (./evm, ./solana) declare their own typed views of the
 * same underlying native module — `requireNativeModule` returns the
 * registered Expo Module instance, so multiple typed accessors over it are
 * safe.
 */
export interface ChainAgnosticBindings {
  isValidMnemonic(mnemonic: string): boolean;
  createWallet(strength: number): Promise<{ mnemonic: string }>;
  deriveAddress(mnemonic: string, coin: number, path: string): Promise<string>;
}

export const Native = requireNativeModule(
  'WalletCoreModule',
) as unknown as ChainAgnosticBindings;
