import Foundation
import SwiftProtobuf

// EVM signing helpers. Called from WalletCoreModule.definition()'s
// `evmSignTransaction` AsyncFunction registration. The JS layer pre-converts
// uint256 values to 0x-hex strings so we don't need a BigInt library here.

enum EvmTxMode {
  case legacy
  case enveloped

  var proto: TW_Ethereum_Proto_TransactionMode {
    switch self {
    case .legacy: return .legacy
    case .enveloped: return .enveloped
    }
  }
}

enum EvmSigning {
  /// Sign an EVM transaction. Returns 0x-prefixed raw signed tx hex.
  static func signTransaction(
    tx: [String: Any],
    mnemonic: String,
    derivationPath: String
  ) throws -> String {
    guard let wallet = HDWallet(mnemonic: mnemonic, passphrase: "") else {
      throw WalletCoreError.invalidMnemonic
    }
    let privateKey = wallet.getKey(coin: .ethereum, derivationPath: derivationPath)

    // Memory hygiene (mirrors cirqle audit W-06): hold the key bytes in a
    // mutable local Data (value-typed copy) and zero-fill it on every exit
    // path after AnySigner.sign returns. wallet-core's PrivateKey object
    // retains its own buffer (deinit-bound, outside our reach) — this is
    // scope-narrowing, not zero-residual.
    var pkData = privateKey.data
    var input = TW_Ethereum_Proto_SigningInput()
    defer {
      pkData.resetBytes(in: 0..<pkData.count)
      input.privateKey = Data()
    }

    let mode = try detectMode(tx)

    input.chainID = try uint256("chainId", tx["chainId"])
    input.nonce = try uint256("nonce", tx["nonce"])
    input.gasLimit = try uint256("gasLimit", tx["gasLimit"])
    input.txMode = mode.proto

    switch mode {
    case .legacy:
      input.gasPrice = try uint256("gasPrice", tx["gasPrice"])
    case .enveloped:
      input.maxFeePerGas = try uint256("maxFeePerGas", tx["maxFeePerGas"])
      input.maxInclusionFeePerGas = try uint256("maxPriorityFeePerGas", tx["maxPriorityFeePerGas"])
    }

    guard let toAddress = tx["to"] as? String else {
      throw WalletCoreError.missingField("to")
    }
    input.toAddress = toAddress
    input.privateKey = pkData

    input.transaction = try buildTransaction(tx)

    // Upstream AnySigner is generic over SigningOutput:
    //   public static func sign<O: Message>(input: SigningInput, coin: CoinType) -> O
    // The return-type annotation drives the inference of O.
    let output: TW_Ethereum_Proto_SigningOutput =
        AnySigner.sign(input: input, coin: .ethereum)
    return "0x" + EvmSigning.toHex(output.encoded)
  }

  // MARK: - Helpers

  private static func detectMode(_ tx: [String: Any]) throws -> EvmTxMode {
    let hasLegacy = tx["gasPrice"] != nil
    let hasFeeMarket = tx["maxFeePerGas"] != nil || tx["maxPriorityFeePerGas"] != nil
    if hasLegacy && hasFeeMarket { throw WalletCoreError.conflictingGasFields }
    if hasFeeMarket { return .enveloped }
    if hasLegacy { return .legacy }
    throw WalletCoreError.missingGasFields
  }

  /// Parse a uint256 input as minimal big-endian bytes (RLP-friendly):
  ///   0     -> []          (empty)
  ///   1     -> [0x01]
  ///   256   -> [0x01, 0x00]
  /// Accepts 0x-prefixed hex (preferred) or numeric (NSNumber).
  private static func uint256(_ field: String, _ value: Any?) throws -> Data {
    guard let value = value else { throw WalletCoreError.missingField(field) }
    let str: String
    switch value {
    case let s as String:
      str = s
    case let n as NSNumber:
      // Bridge usually delivers JS numbers as NSNumber. Format as hex.
      str = "0x" + String(n.uint64Value, radix: 16)
    default:
      throw WalletCoreError.invalidField(field, "expected hex string or number")
    }
    guard str.lowercased().hasPrefix("0x") else {
      throw WalletCoreError.invalidField(field, "expected 0x-prefixed hex")
    }
    let hex = String(str.dropFirst(2))
    if hex.isEmpty { return Data() }
    // Strip ALL leading zeros (minimal encoding).
    var i = hex.startIndex
    while i < hex.endIndex, hex[i] == "0" {
      i = hex.index(after: i)
    }
    let trimmed = String(hex[i..<hex.endIndex])
    if trimmed.isEmpty { return Data() }
    let padded = trimmed.count.isMultiple(of: 2) ? trimmed : "0" + trimmed
    guard let data = EvmSigning.dataFromHex(padded) else {
      throw WalletCoreError.invalidField(field, "invalid hex (\(str))")
    }
    return data
  }

  private static func buildTransaction(_ tx: [String: Any]) throws -> TW_Ethereum_Proto_Transaction {
    var transaction = TW_Ethereum_Proto_Transaction()
    let value = try uint256("value", tx["value"] ?? "0x0")

    if let dataStr = tx["data"] as? String, !isEmptyHex(dataStr) {
      guard dataStr.lowercased().hasPrefix("0x") else {
        throw WalletCoreError.invalidField("data", "expected 0x-prefixed hex")
      }
      let hex = String(dataStr.dropFirst(2))
      let padded = hex.count.isMultiple(of: 2) ? hex : "0" + hex
      guard let dataBytes = EvmSigning.dataFromHex(padded) else {
        throw WalletCoreError.invalidField("data", "invalid hex (\(dataStr))")
      }
      var contractGeneric = TW_Ethereum_Proto_Transaction.ContractGeneric()
      contractGeneric.amount = value
      contractGeneric.data = dataBytes
      transaction.contractGeneric = contractGeneric
    } else {
      var transfer = TW_Ethereum_Proto_Transaction.Transfer()
      transfer.amount = value
      transaction.transfer = transfer
    }
    return transaction
  }

  private static func isEmptyHex(_ s: String) -> Bool {
    return s.isEmpty || s == "0x" || s == "0X"
  }

  static func dataFromHex(_ hex: String) -> Data? {
    var data = Data(capacity: hex.count / 2)
    var i = hex.startIndex
    while i < hex.endIndex {
      let next = hex.index(i, offsetBy: 2)
      guard let byte = UInt8(hex[i..<next], radix: 16) else { return nil }
      data.append(byte)
      i = next
    }
    return data
  }

  static func toHex(_ data: Data) -> String {
    return data.map { String(format: "%02x", $0) }.joined()
  }
}
