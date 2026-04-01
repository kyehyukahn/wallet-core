// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

#pragma once

#include "TWBase.h"

TW_EXTERN_C_BEGIN

/// Bitcoin SPV Sync — connects to Bitcoin P2P network for lightweight synchronization.
TW_EXPORT_CLASS
struct TWBitcoinSpvSync;

/// Creates a new SPV sync instance for Bitcoin mainnet.
///
/// \param earliestKeyTime Unix timestamp of wallet creation (for sync optimization)
/// \note Null is returned if runtime creation fails
/// \note Returned object needs to be deleted with \TWBitcoinSpvSyncDelete
/// \return Nullable TWBitcoinSpvSync
TW_EXPORT_STATIC_METHOD
struct TWBitcoinSpvSync *_Nullable TWBitcoinSpvSyncCreate(uint32_t earliestKeyTime);

/// Deletes an SPV sync instance and shuts down its runtime.
TW_EXPORT_METHOD
void TWBitcoinSpvSyncDelete(struct TWBitcoinSpvSync *_Nonnull sync);

/// Starts SPV synchronization (DNS resolution, peer connection, block sync).
TW_EXPORT_METHOD
void TWBitcoinSpvSyncStart(struct TWBitcoinSpvSync *_Nonnull sync);

/// Stops SPV synchronization.
TW_EXPORT_METHOD
void TWBitcoinSpvSyncStop(struct TWBitcoinSpvSync *_Nonnull sync);

/// Returns sync progress as a value between 0.0 and 1.0.
TW_EXPORT_PROPERTY
double TWBitcoinSpvSyncProgress(struct TWBitcoinSpvSync *_Nonnull sync);

/// Returns the estimated block height from connected peers.
TW_EXPORT_PROPERTY
uint32_t TWBitcoinSpvSyncEstimatedHeight(struct TWBitcoinSpvSync *_Nonnull sync);

/// Returns the current sync capability.
/// \return 0=FullBip37, 1=CompactFilters, 2=HeaderOnly, 3=Unsupported
TW_EXPORT_PROPERTY
int TWBitcoinSpvSyncCapability(struct TWBitcoinSpvSync *_Nonnull sync);

/// Returns the number of currently connected peers.
TW_EXPORT_PROPERTY
uint32_t TWBitcoinSpvSyncConnectedPeerCount(struct TWBitcoinSpvSync *_Nonnull sync);

/// Returns whether sync is currently running.
TW_EXPORT_PROPERTY
bool TWBitcoinSpvSyncIsRunning(struct TWBitcoinSpvSync *_Nonnull sync);

/// Returns the last synced block height.
TW_EXPORT_PROPERTY
uint32_t TWBitcoinSpvSyncLastBlockHeight(struct TWBitcoinSpvSync *_Nonnull sync);

TW_EXTERN_C_END
