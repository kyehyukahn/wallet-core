package expo.modules.walletcore.chains

import com.google.protobuf.ByteString
import expo.modules.walletcore.WalletCoreError
import java.math.BigInteger
import wallet.core.java.AnySigner
import wallet.core.jni.CoinType
import wallet.core.jni.HDWallet
import wallet.core.jni.proto.Ethereum

private enum class EvmTxMode {
    LEGACY,
    ENVELOPED;

    fun toProto(): Ethereum.TransactionMode = when (this) {
        LEGACY -> Ethereum.TransactionMode.Legacy
        ENVELOPED -> Ethereum.TransactionMode.Enveloped
    }
}

/**
 * EVM signing helpers. Called from WalletCoreModule.definition()'s
 * evmSignTransaction AsyncFunction. JS layer pre-converts uint256 fields
 * to 0x-prefixed hex so we don't need BigInteger arithmetic on input
 * except for the well-bounded hex -> bytes conversion.
 */
internal object EvmSigning {

    fun signTransaction(
        tx: Map<String, Any?>,
        mnemonic: String,
        derivationPath: String,
    ): String {
        val wallet = try {
            HDWallet(mnemonic, "")
        } catch (e: Exception) {
            throw WalletCoreError.InvalidMnemonic()
        }
        val privateKey = wallet.getKey(CoinType.ETHEREUM, derivationPath)

        val mode = detectMode(tx)
        val toAddress = (tx["to"] as? String) ?: throw WalletCoreError.MissingField("to")

        val builder = Ethereum.SigningInput.newBuilder()
            .setChainId(uint256("chainId", tx["chainId"]))
            .setNonce(uint256("nonce", tx["nonce"]))
            .setGasLimit(uint256("gasLimit", tx["gasLimit"]))
            .setTxMode(mode.toProto())
            .setToAddress(toAddress)
            .setPrivateKey(ByteString.copyFrom(privateKey.data()))
            .setTransaction(buildTransaction(tx))

        when (mode) {
            EvmTxMode.LEGACY ->
                builder.gasPrice = uint256("gasPrice", tx["gasPrice"])
            EvmTxMode.ENVELOPED -> {
                builder.maxFeePerGas = uint256("maxFeePerGas", tx["maxFeePerGas"])
                builder.maxInclusionFeePerGas = uint256("maxPriorityFeePerGas", tx["maxPriorityFeePerGas"])
            }
        }

        // Java AnySigner is generic:
        //   <T extends MessageLite> T sign(MessageLite input, CoinType coin, Parser<T>)
        // Pass the protobuf input + parser; AnySigner serialises and parses
        // for us. Earlier scaffold called sign(byte[], CoinType) which is
        // not a real overload (signed bytes round-tripped through plain
        // nativeSign return raw bytes, never a SigningOutput).
        val output: Ethereum.SigningOutput = AnySigner.sign(
            builder.build(),
            CoinType.ETHEREUM,
            Ethereum.SigningOutput.parser(),
        )
        return "0x" + output.encoded.toByteArray().toHexString()
    }

    private fun detectMode(tx: Map<String, Any?>): EvmTxMode {
        val hasLegacy = tx["gasPrice"] != null
        val hasFeeMarket = tx["maxFeePerGas"] != null || tx["maxPriorityFeePerGas"] != null
        if (hasLegacy && hasFeeMarket) throw WalletCoreError.ConflictingGasFields()
        if (hasFeeMarket) return EvmTxMode.ENVELOPED
        if (hasLegacy) return EvmTxMode.LEGACY
        throw WalletCoreError.MissingGasFields()
    }

    /**
     * Minimal big-endian bytes for a uint256 input:
     *   0     -> EMPTY
     *   1     -> [0x01]
     *   256   -> [0x01, 0x00]
     */
    private fun uint256(field: String, value: Any?): ByteString {
        if (value == null) throw WalletCoreError.MissingField(field)
        val str = when (value) {
            is String -> value
            is Number -> "0x" + java.lang.Long.toHexString(value.toLong())
            else -> throw WalletCoreError.InvalidField(field, "expected hex string or number")
        }
        val lower = str.lowercase()
        if (!lower.startsWith("0x")) {
            throw WalletCoreError.InvalidField(field, "expected 0x-prefixed hex")
        }
        val hex = str.substring(2)
        if (hex.isEmpty()) return ByteString.EMPTY
        val trimmed = hex.trimStart('0')
        if (trimmed.isEmpty()) return ByteString.EMPTY
        val n = try {
            BigInteger(trimmed, 16)
        } catch (e: NumberFormatException) {
            throw WalletCoreError.InvalidField(field, "invalid hex ($str)")
        }
        val bytes = n.toByteArray()
        // BigInteger.toByteArray() prepends a sign byte for positive numbers
        // whose top bit is set. Strip that leading 0x00 so encoding stays minimal.
        val minimal = if (bytes.size > 1 && bytes[0] == 0.toByte()) {
            bytes.copyOfRange(1, bytes.size)
        } else {
            bytes
        }
        return ByteString.copyFrom(minimal)
    }

    private fun buildTransaction(tx: Map<String, Any?>): Ethereum.Transaction {
        val value = uint256("value", tx["value"] ?: "0x0")
        val builder = Ethereum.Transaction.newBuilder()

        val dataStr = tx["data"] as? String
        return if (!dataStr.isNullOrEmpty() && dataStr != "0x" && dataStr != "0X") {
            val lower = dataStr.lowercase()
            if (!lower.startsWith("0x")) {
                throw WalletCoreError.InvalidField("data", "expected 0x-prefixed hex")
            }
            val hex = dataStr.substring(2)
            val padded = if (hex.length % 2 == 0) hex else "0$hex"
            val dataBytes = try {
                hexStringToByteArray(padded)
            } catch (e: NumberFormatException) {
                throw WalletCoreError.InvalidField("data", "invalid hex ($dataStr)")
            }
            builder.setContractGeneric(
                Ethereum.Transaction.ContractGeneric.newBuilder()
                    .setAmount(value)
                    .setData(ByteString.copyFrom(dataBytes))
                    .build(),
            ).build()
        } else {
            builder.setTransfer(
                Ethereum.Transaction.Transfer.newBuilder()
                    .setAmount(value)
                    .build(),
            ).build()
        }
    }

    private fun hexStringToByteArray(hex: String): ByteArray {
        return ByteArray(hex.length / 2) { i ->
            Integer.parseInt(hex, i * 2, i * 2 + 2, 16).toByte()
        }
    }

    private fun ByteArray.toHexString(): String =
        joinToString("") { "%02x".format(it) }
}
