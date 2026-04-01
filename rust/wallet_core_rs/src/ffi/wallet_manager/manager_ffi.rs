// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

#![allow(clippy::missing_safety_doc)]

use bitcoin::bip32::ExtendedPubKey;
use std::str::FromStr;
use std::sync::Arc;
use tw_memory::ffi::tw_string::TWString;
use tw_memory::ffi::RawPtrTrait;
use tw_misc::try_or_else;
use tw_wallet_manager::persistence::test_persistence::InMemoryPersistence;
use tw_wallet_manager::recovery::RecoveryAction;
use tw_wallet_manager::wallet_manager::BitcoinWalletManager;

/// Opaque handle wrapping a `BitcoinWalletManager` for FFI consumers.
pub struct TWBitcoinWalletManagerImpl {
    pub(crate) manager: BitcoinWalletManager,
}

impl RawPtrTrait for TWBitcoinWalletManagerImpl {}

/// Creates a new `TWBitcoinWalletManagerImpl` from the given xpub string.
/// Returns null on invalid xpub or creation failure.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_create(
    xpub: *const TWString,
) -> *mut TWBitcoinWalletManagerImpl {
    let xpub_str = try_or_else!(TWString::from_ptr_as_ref(xpub), std::ptr::null_mut);
    let xpub_str = try_or_else!(xpub_str.as_str(), std::ptr::null_mut);
    let xpub_key = try_or_else!(ExtendedPubKey::from_str(xpub_str), std::ptr::null_mut);
    let persistence = Arc::new(InMemoryPersistence::new());
    let manager = try_or_else!(
        BitcoinWalletManager::new(xpub_key, persistence),
        std::ptr::null_mut
    );
    TWBitcoinWalletManagerImpl { manager }.into_ptr()
}

/// Deletes (frees) the given `TWBitcoinWalletManagerImpl`.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_delete(
    manager: *mut TWBitcoinWalletManagerImpl,
) {
    let _ = TWBitcoinWalletManagerImpl::from_ptr(manager);
}

/// Returns the wallet balance in satoshis.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_balance(
    manager: *const TWBitcoinWalletManagerImpl,
) -> u64 {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager), || 0);
    mgr.manager.balance()
}

/// Returns the first unused receiving address as a `TWString`, or null on error.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_receive_address(
    manager: *const TWBitcoinWalletManagerImpl,
) -> *mut TWString {
    let mgr = try_or_else!(
        TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager),
        std::ptr::null_mut
    );
    let addr = try_or_else!(mgr.manager.receive_address(), std::ptr::null_mut);
    TWString::from(addr.to_string()).into_ptr()
}

/// Returns the first unused change address as a `TWString`, or null on error.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_change_address(
    manager: *const TWBitcoinWalletManagerImpl,
) -> *mut TWString {
    let mgr = try_or_else!(
        TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager),
        std::ptr::null_mut
    );
    let addr = try_or_else!(mgr.manager.change_address(), std::ptr::null_mut);
    TWString::from(addr.to_string()).into_ptr()
}

/// Returns the current fee rate in satoshis per kilobyte.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_fee_rate(
    manager: *const TWBitcoinWalletManagerImpl,
) -> u64 {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager), || 0);
    mgr.manager.fee_rate()
}

/// Sets the fee rate in satoshis per kilobyte.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_set_fee_rate(
    manager: *mut TWBitcoinWalletManagerImpl,
    fee_rate: u64,
) {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_mut(manager), || {});
    mgr.manager.set_fee_rate(fee_rate);
}

/// Estimates the fee for sending the given amount in satoshis.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_estimate_fee(
    manager: *const TWBitcoinWalletManagerImpl,
    amount: u64,
) -> u64 {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager), || 0);
    mgr.manager.estimate_fee(amount)
}

/// Returns the maximum spendable amount in satoshis.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_max_spendable(
    manager: *const TWBitcoinWalletManagerImpl,
) -> u64 {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager), || 0);
    mgr.manager.max_spendable()
}

/// Returns the minimum output amount (dust limit) in satoshis.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_min_output_amount(
    manager: *const TWBitcoinWalletManagerImpl,
) -> u64 {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager), || 0);
    mgr.manager.min_output_amount()
}

/// Saves the current wallet state to persistence. Returns `true` on success.
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_save_state(
    manager: *const TWBitcoinWalletManagerImpl,
) -> bool {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_ref(manager), || {
        false
    });
    mgr.manager.save_state().is_ok()
}

/// Initializes the wallet manager by determining the recovery action from persisted state.
///
/// Returns:
///   0 = FreshStart
///   1 = ResumeNormal
///   2 = WalletRescan
///   3 = FullResync
///  -1 = Error
#[no_mangle]
pub unsafe extern "C" fn tw_bitcoin_wallet_manager_initialize(
    manager: *mut TWBitcoinWalletManagerImpl,
) -> i32 {
    let mgr = try_or_else!(TWBitcoinWalletManagerImpl::from_ptr_as_mut(manager), || -1);
    match mgr.manager.initialize() {
        Ok(RecoveryAction::FreshStart) => 0,
        Ok(RecoveryAction::ResumeNormal { .. }) => 1,
        Ok(RecoveryAction::WalletRescan { .. }) => 2,
        Ok(RecoveryAction::FullResync { .. }) => 3,
        Err(_) => -1,
    }
}
