package expo.modules.walletcore

import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import expo.modules.walletcore.chains.EvmSigning
import wallet.core.jni.AnyAddress
import wallet.core.jni.CoinType
import wallet.core.jni.CoinTypeConfiguration
import wallet.core.jni.HDWallet
import wallet.core.jni.Mnemonic

// Chain-agnostic Expo Module bridging the kyehyukahn/wallet-core fork to
// React Native. Per-chain signing methods are appended in the same
// `definition()` body by calling static helpers in chains/<Chain>Signing.kt.

class WalletCoreModule : Module() {
    override fun definition() = ModuleDefinition {
        Name("WalletCoreModule")

        // ── chain-agnostic primitives ──

        Function("isValidMnemonic") { mnemonic: String ->
            Mnemonic.isValid(mnemonic)
        }

        // strength: 128 → 12 words, 256 → 24 words.
        AsyncFunction("createWallet") { strength: Int ->
            val wallet = HDWallet(strength, "")
            mapOf("mnemonic" to wallet.mnemonic())
        }

        // Derive an address for any registered CoinType + derivation path.
        AsyncFunction("deriveAddress") { mnemonic: String, coin: Int, path: String ->
            val coinType = CoinType.createFromValue(coin)
                ?: throw WalletCoreError.UnsupportedCoinType(coin)
            val wallet = HDWallet(mnemonic, "")
            val privateKey = wallet.getKey(coinType, path)
            val pubKeyType = CoinTypeConfiguration.getPublicKeyType(coinType)
            val publicKey = privateKey.getPublicKeyByType(pubKeyType)
            AnyAddress(publicKey, coinType).description()
        }

        // ── EVM (chains/EvmSigning.kt) ──

        AsyncFunction("evmSignTransaction") { tx: Map<String, Any?>, mnemonic: String, path: String ->
            EvmSigning.signTransaction(tx, mnemonic, path)
        }

        // ── per-chain signing methods are added by chains/<Chain>Signing.kt ──
        // commit 7: Solana (solanaSignTransfer)
    }
}

sealed class WalletCoreError(message: String) : Exception(message) {
    class InvalidMnemonic : WalletCoreError("invalid BIP39 mnemonic")
    class UnsupportedCoinType(coin: Int) : WalletCoreError("unsupported CoinType id: $coin")
    class MissingField(field: String) : WalletCoreError("missing required field: $field")
    class InvalidField(field: String, reason: String) : WalletCoreError("invalid field $field: $reason")
    class ConflictingGasFields : WalletCoreError("conflicting gas fields: specify gasPrice OR max*PerGas, not both")
    class MissingGasFields : WalletCoreError("missing gas fields: provide either gasPrice or max*PerGas")
    class UnsupportedTransactionType(type: Int) : WalletCoreError("unsupported EVM transaction type: $type")
}
