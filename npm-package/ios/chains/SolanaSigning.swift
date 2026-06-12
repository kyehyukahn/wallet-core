import Foundation
import SwiftProtobuf

// Solana signing helpers. Called from WalletCoreModule.definition()'s
// `solanaSignTransfer` AsyncFunction. wallet-core's Solana support covers
// many instruction types; v0.2.0 binds only the basic transfer to validate
// the multi-chain package layout. Additional instructions (token transfer,
// stake, vote, etc.) follow the same pattern.

enum SolanaSigning {
  /// Sign a SOL transfer. Returns the wallet-core SigningOutput.encoded string
  /// (an encoded Solana transaction, ready for `sendTransaction` RPC).
  static func signTransfer(
    tx: [String: Any],
    mnemonic: String,
    derivationPath: String
  ) throws -> String {
    guard let wallet = HDWallet(mnemonic: mnemonic, passphrase: "") else {
      throw WalletCoreError.invalidMnemonic
    }
    let privateKey = wallet.getKey(coin: .solana, derivationPath: derivationPath)

    // Memory hygiene — same W-06 pattern as EvmSigning.swift: zero-fill the
    // extracted key bytes on every exit path after AnySigner.sign returns.
    var pkData = privateKey.data
    var input = TW_Solana_Proto_SigningInput()
    defer {
      pkData.resetBytes(in: 0..<pkData.count)
      input.privateKey = Data()
    }

    guard let recipient = tx["recipient"] as? String else {
      throw WalletCoreError.missingField("recipient")
    }
    guard let lamportsValue = tx["lamports"] else {
      throw WalletCoreError.missingField("lamports")
    }
    let lamports: UInt64
    switch lamportsValue {
    case let s as String:
      guard let parsed = UInt64(s) else {
        throw WalletCoreError.invalidField("lamports", "not a valid uint64 (\(s))")
      }
      lamports = parsed
    case let n as NSNumber:
      lamports = n.uint64Value
    default:
      throw WalletCoreError.invalidField("lamports", "expected uint64 string or number")
    }
    guard let recentBlockhash = tx["recentBlockhash"] as? String else {
      throw WalletCoreError.missingField("recentBlockhash")
    }

    input.privateKey = pkData
    input.recentBlockhash = recentBlockhash
    input.transferTransaction = TW_Solana_Proto_Transfer.with {
      $0.recipient = recipient
      $0.value = lamports
      if let memo = tx["memo"] as? String {
        $0.memo = memo
      }
    }

    // See EvmSigning.swift for the AnySigner.sign<Output>(input:coin:) generic.
    let output: TW_Solana_Proto_SigningOutput =
        AnySigner.sign(input: input, coin: .solana)
    return output.encoded
  }
}
