/**
 * BIP-44 coin type IDs that the native module exposes via wallet-core's
 * CoinType enum. Numeric values match SLIP-44 / wallet-core's CoinType.h
 * and are what the native side reconstructs (Swift: `CoinType(rawValue:)`,
 * Kotlin: `CoinType.createFromValue(...)`).
 *
 * v0.2.0 ships native signing for Ethereum and Solana. Other coins can be
 * added by extending this enum + a `chains/<Chain>Signing.{swift,kt}` pair
 * + a subpath under the package (e.g. `cosmos/`).
 */
export enum CoinType {
  Ethereum = 60,
  Solana = 501,
}
