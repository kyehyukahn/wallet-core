package expo.modules.walletcore.chains

import com.google.protobuf.ByteString
import expo.modules.walletcore.WalletCoreError
import wallet.core.java.AnySigner
import wallet.core.jni.CoinType
import wallet.core.jni.HDWallet
import wallet.core.jni.proto.Solana

/**
 * Solana signing helpers. Called from WalletCoreModule.definition()'s
 * solanaSignTransfer AsyncFunction. v0.2.0 binds the basic SOL transfer
 * to validate the multi-chain package layout; additional instructions
 * follow the same pattern.
 */
internal object SolanaSigning {

    fun signTransfer(
        tx: Map<String, Any?>,
        mnemonic: String,
        derivationPath: String,
    ): String {
        val wallet = try {
            HDWallet(mnemonic, "")
        } catch (e: Exception) {
            throw WalletCoreError.InvalidMnemonic()
        }
        val privateKey = wallet.getKey(CoinType.SOLANA, derivationPath)

        val recipient = (tx["recipient"] as? String)
            ?: throw WalletCoreError.MissingField("recipient")
        val recentBlockhash = (tx["recentBlockhash"] as? String)
            ?: throw WalletCoreError.MissingField("recentBlockhash")

        // Java's signed long can hold the full uint64 bit pattern when parsed
        // via parseUnsignedLong; the protobuf wire format treats it as uint64.
        val lamports: Long = when (val raw = tx["lamports"]) {
            null -> throw WalletCoreError.MissingField("lamports")
            is String -> try {
                java.lang.Long.parseUnsignedLong(raw)
            } catch (e: NumberFormatException) {
                throw WalletCoreError.InvalidField("lamports", "not a valid uint64 ($raw)")
            }
            is Number -> raw.toLong()
            else -> throw WalletCoreError.InvalidField("lamports", "expected uint64 string or number")
        }

        val transferBuilder = Solana.Transfer.newBuilder()
            .setRecipient(recipient)
            .setValue(lamports)
        (tx["memo"] as? String)?.let { transferBuilder.memo = it }

        val input = Solana.SigningInput.newBuilder()
            .setPrivateKey(ByteString.copyFrom(privateKey.data()))
            .setRecentBlockhash(recentBlockhash)
            .setTransferTransaction(transferBuilder.build())
            .build()

        val outputBytes = AnySigner.sign(input.toByteArray(), CoinType.SOLANA)
        val output = Solana.SigningOutput.parseFrom(outputBytes)
        return output.encoded
    }
}
