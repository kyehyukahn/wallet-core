import ExpoModulesCore

// Chain-agnostic Expo Module bridging the kyehyukahn/wallet-core fork to
// React Native. Per-chain signing methods (evm*, solana*) are appended in
// the same `definition()` body — Expo's ModuleDefinition DSL must run in
// one place, so each chain provides static helpers in chains/<Chain>Signing.swift
// that this file calls from inside the DSL block.

public class WalletCoreModule: Module {
  public func definition() -> ModuleDefinition {
    Name("WalletCoreModule")

    // ── chain-agnostic primitives ──

    // Mnemonic validation is pure (no derivation), so it's a synchronous
    // Function rather than AsyncFunction.
    Function("isValidMnemonic") { (mnemonic: String) -> Bool in
      Mnemonic.isValid(mnemonic: mnemonic)
    }

    // strength: 128 → 12 words, 256 → 24 words. wallet-core enforces.
    AsyncFunction("createWallet") { (strength: Int) -> [String: String] in
      guard let wallet = HDWallet(strength: Int32(strength), passphrase: "") else {
        throw WalletCoreError.hdWalletCreationFailed
      }
      return ["mnemonic": wallet.mnemonic]
    }

    // Derive an address for any registered coin type + derivation path.
    // Per-chain subpaths (evm, solana) call this with their CoinType numeric.
    AsyncFunction("deriveAddress") { (mnemonic: String, coin: Int, path: String) -> String in
      guard let wallet = HDWallet(mnemonic: mnemonic, passphrase: "") else {
        throw WalletCoreError.invalidMnemonic
      }
      guard let coinType = CoinType(rawValue: UInt32(coin)) else {
        throw WalletCoreError.unsupportedCoinType(coin)
      }
      let privateKey = wallet.getKey(coin: coinType, derivationPath: path)
      let pubKeyType = CoinTypeConfiguration.getPublicKeyType(coin: coinType)
      let publicKey = privateKey.getPublicKeyByType(pubkeyType: pubKeyType)
      return AnyAddress(publicKey: publicKey, coin: coinType).description
    }

    // ── EVM (chains/EvmSigning.swift) ──

    AsyncFunction("evmSignTransaction") { (tx: [String: Any], mnemonic: String, path: String) -> String in
      try EvmSigning.signTransaction(tx: tx, mnemonic: mnemonic, derivationPath: path)
    }

    // ── per-chain signing methods are added by chains/<Chain>Signing.swift ──
    // commit 7: Solana (solanaSignTransfer)
  }
}

enum WalletCoreError: Error, LocalizedError {
  case hdWalletCreationFailed
  case invalidMnemonic
  case unsupportedCoinType(Int)
  case missingField(String)
  case invalidField(String, String)
  case conflictingGasFields
  case missingGasFields
  case unsupportedTransactionType(Int)

  var errorDescription: String? {
    switch self {
    case .hdWalletCreationFailed:
      return "HDWallet creation failed (invalid strength?)"
    case .invalidMnemonic:
      return "invalid BIP39 mnemonic"
    case .unsupportedCoinType(let n):
      return "unsupported CoinType id: \(n)"
    case .missingField(let f):
      return "missing required field: \(f)"
    case .invalidField(let f, let reason):
      return "invalid field \(f): \(reason)"
    case .conflictingGasFields:
      return "conflicting gas fields: specify gasPrice OR max*PerGas, not both"
    case .missingGasFields:
      return "missing gas fields: provide either gasPrice or max*PerGas"
    case .unsupportedTransactionType(let t):
      return "unsupported EVM transaction type: \(t)"
    }
  }
}
