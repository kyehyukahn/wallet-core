// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

#![allow(clippy::missing_safety_doc)]

use std::sync::atomic::{AtomicBool, Ordering};
use tw_memory::ffi::RawPtrTrait;
use tw_misc::try_or_else;
use tw_spv::bitcoin_chain::BitcoinMainnet;
use tw_spv::chain::SpvChain;
use tw_spv::events::SyncCapability;
use tw_spv::peer_manager::PeerPool;
use tw_spv::runtime::SpvRuntime;
use tw_spv::sync_manager::SyncState;
use tw_spv::tokio_runtime::TokioRuntime;

/// Opaque handle for Bitcoin SPV sync exposed via FFI.
pub struct TWBitcoinSpvSyncImpl {
    runtime: TokioRuntime,
    peer_pool: PeerPool,
    sync_state: SyncState,
    is_running: AtomicBool,
    chain: BitcoinMainnet,
}

impl RawPtrTrait for TWBitcoinSpvSyncImpl {}

/// Creates a new `TWBitcoinSpvSync` instance.
///
/// \param earliest_key_time Unix timestamp of the earliest wallet key creation time.
/// \return A non-null pointer to the new instance. Caller must eventually call
///         `tw_bitcoin_spv_sync_delete` to free the memory.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_create(
    earliest_key_time: u32,
) -> *mut TWBitcoinSpvSyncImpl {
    let chain = BitcoinMainnet;
    let sync_state = SyncState::new(&chain, earliest_key_time);
    let peer_pool = PeerPool::new();
    let runtime = TokioRuntime::new();

    let spv = TWBitcoinSpvSyncImpl {
        runtime,
        peer_pool,
        sync_state,
        is_running: AtomicBool::new(false),
        chain,
    };
    spv.into_ptr()
}

/// Deletes a `TWBitcoinSpvSync` instance and frees its memory.
///
/// \param sync Previously created instance (may be null, in which case this is a no-op).
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_delete(sync: *mut TWBitcoinSpvSyncImpl) {
    // Taking ownership via `from_ptr` drops the value at the end of this scope.
    let _ = TWBitcoinSpvSyncImpl::from_ptr(sync);
}

/// Starts SPV synchronisation.
///
/// Sets `is_running` to true, begins header sync, and spawns an async task
/// that resolves DNS seeds to demonstrate the tokio runtime is functional.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_start(sync: *mut TWBitcoinSpvSyncImpl) {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_mut(sync), || ());

    if spv.is_running.load(Ordering::SeqCst) {
        return;
    }
    spv.is_running.store(true, Ordering::SeqCst);
    let _ = spv.sync_state.start_sync();

    // Spawn a DNS resolution task to demonstrate the runtime works.
    let dns_seeds: Vec<String> = spv
        .chain
        .dns_seeds()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let port = spv.chain.default_port();
    spv.runtime.spawn(Box::pin(async move {
        for seed in &dns_seeds {
            let addr = format!("{}:{}", seed, port);
            let resolved = tokio::net::lookup_host(addr).await;
            if let Ok(addrs) = resolved {
                for socket_addr in addrs {
                    let _ = socket_addr; // Full peer connection in future iteration
                }
            }
        }
    }));
}

/// Stops SPV synchronisation.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_stop(sync: *mut TWBitcoinSpvSyncImpl) {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_mut(sync), || ());

    spv.is_running.store(false, Ordering::SeqCst);
    spv.sync_state.stop_sync();
}

/// Returns the current sync progress as a value between 0.0 and 1.0.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
/// \return Progress in the range [0.0, 1.0]. Returns 0.0 on null input.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_progress(sync: *const TWBitcoinSpvSyncImpl) -> f64 {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_ref(sync), || 0.0);
    spv.sync_state
        .sync_progress(spv.peer_pool.estimated_height())
}

/// Returns the estimated blockchain height based on connected peers.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
/// \return Estimated height, or 0 on null input.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_estimated_height(
    sync: *const TWBitcoinSpvSyncImpl,
) -> u32 {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_ref(sync), || 0);
    spv.peer_pool.estimated_height()
}

/// Returns the current sync capability as an integer.
///
/// Values: FullBip37=0, CompactFilters=1, HeaderOnly=2, Unsupported=3.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
/// \return Capability as i32. Returns 3 (Unsupported) on null input.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_capability(sync: *const TWBitcoinSpvSyncImpl) -> i32 {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_ref(sync), || 3);
    match spv.peer_pool.capability() {
        SyncCapability::FullBip37 => 0,
        SyncCapability::CompactFilters => 1,
        SyncCapability::HeaderOnly => 2,
        SyncCapability::Unsupported => 3,
    }
}

/// Returns the number of currently connected peers.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
/// \return Connected peer count, or 0 on null input.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_connected_peer_count(
    sync: *const TWBitcoinSpvSyncImpl,
) -> u32 {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_ref(sync), || 0);
    spv.peer_pool.connected_count() as u32
}

/// Returns whether synchronisation is currently running.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
/// \return `true` if running, `false` if stopped or on null input.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_is_running(sync: *const TWBitcoinSpvSyncImpl) -> bool {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_ref(sync), || false);
    spv.is_running.load(Ordering::SeqCst)
}

/// Returns the last synced block height.
///
/// \param sync Non-null pointer to a `TWBitcoinSpvSync` instance.
/// \return Last block height, or 0 on null input.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_spv_sync_last_block_height(
    sync: *const TWBitcoinSpvSyncImpl,
) -> u32 {
    let spv = try_or_else!(TWBitcoinSpvSyncImpl::from_ptr_as_ref(sync), || 0);
    spv.sync_state.last_height()
}
