// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

#pragma once

#include "TWBase.h"
#include "TWString.h"

TW_EXTERN_C_BEGIN

/// Bitcoin Wallet Manager — manages wallet state, addresses, UTXOs, and fee estimation.
TW_EXPORT_CLASS
struct TWBitcoinWalletManager;

/// Creates a new Bitcoin wallet manager from an extended public key (xpub).
///
/// \param xpub Non-null BIP32 extended public key string
/// \note Null is returned if the xpub is invalid
/// \note Returned object needs to be deleted with \TWBitcoinWalletManagerDelete
/// \return Nullable TWBitcoinWalletManager
TW_EXPORT_STATIC_METHOD
struct TWBitcoinWalletManager *_Nullable TWBitcoinWalletManagerCreate(TWString *_Nonnull xpub);

/// Deletes a wallet manager instance.
TW_EXPORT_METHOD
void TWBitcoinWalletManagerDelete(struct TWBitcoinWalletManager *_Nonnull manager);

/// Initializes wallet from persisted state or starts fresh.
/// \return 0=FreshStart, 1=ResumeNormal, 2=WalletRescan, 3=FullResync, -1=Error
TW_EXPORT_METHOD
int TWBitcoinWalletManagerInitialize(struct TWBitcoinWalletManager *_Nonnull manager);

/// Returns the current wallet balance in satoshis.
TW_EXPORT_PROPERTY
uint64_t TWBitcoinWalletManagerBalance(struct TWBitcoinWalletManager *_Nonnull manager);

/// Returns the first unused receive address.
TW_EXPORT_PROPERTY
TWString *_Nullable TWBitcoinWalletManagerReceiveAddress(struct TWBitcoinWalletManager *_Nonnull manager);

/// Returns the first unused change address.
TW_EXPORT_PROPERTY
TWString *_Nullable TWBitcoinWalletManagerChangeAddress(struct TWBitcoinWalletManager *_Nonnull manager);

/// Returns the current fee rate in satoshis per kilobyte.
TW_EXPORT_PROPERTY
uint64_t TWBitcoinWalletManagerFeeRate(struct TWBitcoinWalletManager *_Nonnull manager);

/// Sets the fee rate in satoshis per kilobyte.
TW_EXPORT_METHOD
void TWBitcoinWalletManagerSetFeeRate(struct TWBitcoinWalletManager *_Nonnull manager, uint64_t feeRate);

/// Estimates the transaction fee for sending the given amount.
TW_EXPORT_METHOD
uint64_t TWBitcoinWalletManagerEstimateFee(struct TWBitcoinWalletManager *_Nonnull manager, uint64_t amount);

/// Returns the maximum amount that can be sent after fees.
TW_EXPORT_PROPERTY
uint64_t TWBitcoinWalletManagerMaxSpendable(struct TWBitcoinWalletManager *_Nonnull manager);

/// Returns the minimum economically viable output amount at current fee rate.
TW_EXPORT_PROPERTY
uint64_t TWBitcoinWalletManagerMinOutputAmount(struct TWBitcoinWalletManager *_Nonnull manager);

/// Saves current wallet state to persistence. Returns true on success.
TW_EXPORT_METHOD
bool TWBitcoinWalletManagerSaveState(struct TWBitcoinWalletManager *_Nonnull manager);

TW_EXTERN_C_END
