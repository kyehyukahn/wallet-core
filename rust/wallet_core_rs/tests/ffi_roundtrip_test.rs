// SPDX-License-Identifier: Apache-2.0
//
// FFI roundtrip tests: verify C ABI functions work when called from Rust.
// Simulates what an iOS/Android app would do through the C interface.

#[cfg(feature = "spv")]
mod wallet_manager_ffi_tests {
    use tw_memory::ffi::tw_string::TWString;
    use tw_memory::ffi::RawPtrTrait;
    use wallet_core_rs::ffi::wallet_manager::manager_ffi::*;

    // BIP32 test vector xpub
    const TEST_XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

    unsafe fn create_tw_string(s: &str) -> *mut TWString {
        TWString::from(s.to_string()).into_ptr()
    }

    #[test]
    fn test_ffi_wallet_manager_create_and_delete() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);
            assert!(
                !manager.is_null(),
                "manager should not be null for valid xpub"
            );

            tw_bitcoin_wallet_manager_delete(manager);

            // Cleanup xpub string
            let _ = TWString::from_ptr(xpub);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_create_invalid_xpub() {
        unsafe {
            let bad_xpub = create_tw_string("not-a-valid-xpub");
            let manager = tw_bitcoin_wallet_manager_create(bad_xpub);
            assert!(manager.is_null(), "manager should be null for invalid xpub");

            let _ = TWString::from_ptr(bad_xpub);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_balance_initially_zero() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);
            assert!(!manager.is_null());

            let balance = tw_bitcoin_wallet_manager_balance(manager);
            assert_eq!(balance, 0, "initial balance should be 0");

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_receive_address() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);
            assert!(!manager.is_null());

            let addr_ptr = tw_bitcoin_wallet_manager_receive_address(manager);
            assert!(!addr_ptr.is_null(), "receive address should not be null");

            // Verify it's a non-empty string
            let addr = TWString::from_ptr_as_ref(addr_ptr).unwrap();
            let addr_str = addr.as_str().unwrap();
            assert!(!addr_str.is_empty(), "address should not be empty");

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
            let _ = TWString::from_ptr(addr_ptr);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_change_address_differs_from_receive() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);

            let recv_ptr = tw_bitcoin_wallet_manager_receive_address(manager);
            let change_ptr = tw_bitcoin_wallet_manager_change_address(manager);
            assert!(!recv_ptr.is_null());
            assert!(!change_ptr.is_null());

            let recv = TWString::from_ptr_as_ref(recv_ptr)
                .unwrap()
                .as_str()
                .unwrap();
            let change = TWString::from_ptr_as_ref(change_ptr)
                .unwrap()
                .as_str()
                .unwrap();
            assert_ne!(recv, change, "receive and change addresses should differ");

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
            let _ = TWString::from_ptr(recv_ptr);
            let _ = TWString::from_ptr(change_ptr);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_fee_rate() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);

            // Default fee rate
            let fee = tw_bitcoin_wallet_manager_fee_rate(manager);
            assert!(fee > 0, "default fee rate should be > 0");

            // Set new fee rate
            tw_bitcoin_wallet_manager_set_fee_rate(manager, 20000);
            let fee = tw_bitcoin_wallet_manager_fee_rate(manager);
            assert_eq!(fee, 20000);

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_fee_estimation() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);

            // Empty wallet: max spendable = 0
            let max = tw_bitcoin_wallet_manager_max_spendable(manager);
            assert_eq!(max, 0);

            // Min output should be at least 546 (dust limit)
            let min = tw_bitcoin_wallet_manager_min_output_amount(manager);
            assert!(min >= 546);

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_save_state() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);

            let saved = tw_bitcoin_wallet_manager_save_state(manager);
            assert!(saved, "save should succeed");

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
        }
    }

    #[test]
    fn test_ffi_wallet_manager_initialize() {
        unsafe {
            let xpub = create_tw_string(TEST_XPUB);
            let manager = tw_bitcoin_wallet_manager_create(xpub);

            let action = tw_bitcoin_wallet_manager_initialize(manager);
            assert_eq!(action, 0, "fresh start should return 0");

            tw_bitcoin_wallet_manager_delete(manager);
            let _ = TWString::from_ptr(xpub);
        }
    }
}

#[cfg(feature = "spv")]
mod spv_sync_ffi_tests {
    use wallet_core_rs::ffi::spv::sync_ffi::*;

    #[test]
    fn test_ffi_spv_sync_create_and_delete() {
        unsafe {
            let sync = tw_bitcoin_spv_sync_create(1700000000);
            assert!(!sync.is_null(), "sync should not be null");

            tw_bitcoin_spv_sync_delete(sync);
        }
    }

    #[test]
    fn test_ffi_spv_sync_initial_state() {
        unsafe {
            let sync = tw_bitcoin_spv_sync_create(1700000000);

            assert!(
                !tw_bitcoin_spv_sync_is_running(sync),
                "should not be running initially"
            );
            assert_eq!(tw_bitcoin_spv_sync_connected_peer_count(sync), 0);
            assert_eq!(tw_bitcoin_spv_sync_estimated_height(sync), 0);
            assert_eq!(tw_bitcoin_spv_sync_last_block_height(sync), 0);
            assert_eq!(
                tw_bitcoin_spv_sync_capability(sync),
                3,
                "should be Unsupported (3)"
            );

            let progress = tw_bitcoin_spv_sync_progress(sync);
            assert!((0.0..=1.0).contains(&progress));

            tw_bitcoin_spv_sync_delete(sync);
        }
    }

    #[test]
    fn test_ffi_spv_sync_start_stop() {
        unsafe {
            let sync = tw_bitcoin_spv_sync_create(1700000000);

            tw_bitcoin_spv_sync_start(sync);
            assert!(
                tw_bitcoin_spv_sync_is_running(sync),
                "should be running after start"
            );

            tw_bitcoin_spv_sync_stop(sync);
            assert!(
                !tw_bitcoin_spv_sync_is_running(sync),
                "should not be running after stop"
            );

            tw_bitcoin_spv_sync_delete(sync);
        }
    }

    #[test]
    fn test_ffi_spv_sync_double_start_is_safe() {
        unsafe {
            let sync = tw_bitcoin_spv_sync_create(1700000000);

            tw_bitcoin_spv_sync_start(sync);
            tw_bitcoin_spv_sync_start(sync); // should not panic or crash
            assert!(tw_bitcoin_spv_sync_is_running(sync));

            tw_bitcoin_spv_sync_stop(sync);
            tw_bitcoin_spv_sync_delete(sync);
        }
    }

    #[test]
    fn test_ffi_spv_sync_stop_without_start_is_safe() {
        unsafe {
            let sync = tw_bitcoin_spv_sync_create(1700000000);

            tw_bitcoin_spv_sync_stop(sync); // should not panic
            assert!(!tw_bitcoin_spv_sync_is_running(sync));

            tw_bitcoin_spv_sync_delete(sync);
        }
    }
}
