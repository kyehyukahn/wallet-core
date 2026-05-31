import { CoinType } from './CoinType';
import { Native } from './native';

/**
 * BIP-39 / BIP-44 hierarchical deterministic wallet.
 *
 * ⚠ This class holds the mnemonic in JS memory for its lifetime. Apps
 * integrating wallet-core should keep instances short-lived and wrap them
 * behind a domain abstraction (the cirqle-mobile pattern is
 * `lib/wallet/providers/walletCore.ts` exposing an `EvmSigner` interface)
 * so that the rest of the codebase never touches the mnemonic directly.
 *
 * Long-term direction (out of scope for the package itself):
 * mnemonic-holding APIs are intended for first-time create / recover /
 * backup flows. Steady-state signing should keep the mnemonic on the native
 * side via secure storage; the app's domain signer wraps that.
 */
export class HDWallet {
  private constructor(private readonly mnemonic: string) {}

  /**
   * Generate a new mnemonic + wallet.
   * @param strength 128 → 12 words, 256 → 24 words. Default 128.
   */
  static async create(strength: 128 | 256 = 128): Promise<HDWallet> {
    const { mnemonic } = await Native.createWallet(strength);
    return new HDWallet(mnemonic);
  }

  /** Construct from an existing BIP-39 mnemonic. Throws on invalid input. */
  static fromMnemonic(mnemonic: string): HDWallet {
    if (!Native.isValidMnemonic(mnemonic)) {
      throw new Error('invalid BIP39 mnemonic');
    }
    return new HDWallet(mnemonic);
  }

  /** Static mnemonic validator without constructing a wallet. */
  static isValid(mnemonic: string): boolean {
    return Native.isValidMnemonic(mnemonic);
  }

  /** Return the underlying mnemonic. Treat as secret. */
  getMnemonic(): string {
    return this.mnemonic;
  }

  /** Derive an address for the given coin + BIP-44 derivation path. */
  deriveAddress(coin: CoinType, derivationPath: string): Promise<string> {
    return Native.deriveAddress(this.mnemonic, coin, derivationPath);
  }
}
