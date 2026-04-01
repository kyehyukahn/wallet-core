// SPDX-License-Identifier: Apache-2.0
//
// End-to-end integration test verifying all tw_wallet_manager components work together.

use bitcoin::bip32::ExtendedPubKey;
use std::str::FromStr;
use std::sync::Arc;
use tw_hash::H256;
use tw_utxo::transaction::transaction_parts::OutPoint;
use tw_wallet_manager::bitcoin_wallet_chain::BitcoinMainnetWallet;
use tw_wallet_manager::events::WalletEvent;
use tw_wallet_manager::persistence::test_persistence::InMemoryPersistence;
use tw_wallet_manager::persistence::Persistence;
use tw_wallet_manager::recovery::{determine_recovery_action, RecoveryAction};
use tw_wallet_manager::types::{UtxoRecord, WalletTransaction};
use tw_wallet_manager::wallet::Wallet;
use tw_wallet_manager::wallet_manager::BitcoinWalletManager;

fn test_xpub() -> ExtendedPubKey {
    ExtendedPubKey::from_str(
        "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8",
    )
    .unwrap()
}

fn make_utxo(id: u8, value: u64, height: u32) -> UtxoRecord {
    UtxoRecord {
        outpoint: OutPoint {
            hash: H256::from([id; 32]),
            index: 0,
        },
        value,
        script: vec![0x76, 0xa9],
        block_height: Some(height),
        is_change: false,
    }
}

fn make_tx(id: u8, received: u64, sent: u64, height: Option<u32>) -> WalletTransaction {
    WalletTransaction {
        txid: H256::from([id; 32]),
        raw_tx: vec![0x01, 0x00],
        block_height: height,
        timestamp: height.map(|h| 1700000000 + h).unwrap_or(0),
        amount_received: received,
        amount_sent: sent,
        fee: Some(1000),
    }
}

/// Full wallet lifecycle: create -> fund -> spend -> reorg -> persist -> recover
#[test]
fn test_full_wallet_lifecycle() {
    // === Phase: Create wallet ===
    let mut wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();

    assert_eq!(wallet.balance(), 0);
    assert_eq!(wallet.utxo_count(), 0);
    assert_eq!(wallet.transaction_count(), 0);
    assert!(wallet.receive_address().is_some());
    assert!(wallet.change_address().is_some());
    assert_ne!(wallet.receive_address(), wallet.change_address());

    // Verify gap limit: should have 20 external + 20 internal addresses
    assert_eq!(wallet.all_addresses().len(), 40);

    // === Phase: Receive funds ===
    let events = wallet.add_utxo(make_utxo(0x01, 100_000, 800000));
    assert!(events.iter().any(|e| matches!(
        e,
        WalletEvent::BalanceChanged {
            new_balance: 100_000
        }
    )));

    let _events = wallet.add_utxo(make_utxo(0x02, 200_000, 800001));
    assert_eq!(wallet.balance(), 300_000);

    let events = wallet.register_transaction(make_tx(0x01, 100_000, 0, Some(800000)));
    assert!(events
        .iter()
        .any(|e| matches!(e, WalletEvent::TransactionAdded { .. })));

    wallet.register_transaction(make_tx(0x02, 200_000, 0, Some(800001)));
    assert_eq!(wallet.transaction_count(), 2);
    assert_eq!(wallet.total_received(), 300_000);
    assert_eq!(wallet.total_sent(), 0);

    // === Phase: Fee estimation ===
    let fee = wallet.estimate_fee_for_amount(50_000);
    assert!(fee > 0, "fee should be positive");
    assert!(fee < 50_000, "fee should be less than amount");

    let max = wallet.max_spendable();
    assert!(max > 0);
    assert!(max < 300_000, "max should be less than total balance");
    assert!(max > 280_000, "max should be close to total balance");

    let min_output = wallet.min_output_amount();
    assert!(min_output >= 546, "min output should be at least 546 sat");

    // === Phase: Spend ===
    let events = wallet.spend_utxo(&OutPoint {
        hash: H256::from([0x01; 32]),
        index: 0,
    });
    assert_eq!(wallet.balance(), 200_000);
    assert!(events.iter().any(|e| matches!(
        e,
        WalletEvent::BalanceChanged {
            new_balance: 200_000
        }
    )));

    // === Phase: Chain reorg ===
    // Add a UTXO at a higher height
    wallet.add_utxo(make_utxo(0x03, 50_000, 800100));
    assert_eq!(wallet.balance(), 250_000);

    // Reorg at height 800050 -- should remove UTXO at 800100
    let events = wallet.handle_reorg(800050);
    assert_eq!(wallet.balance(), 200_000); // only 800001 UTXO remains
    assert!(events
        .iter()
        .any(|e| matches!(e, WalletEvent::BalanceChanged { .. })));

    // === Phase: Fee rate update ===
    assert_eq!(wallet.fee_rate(), 10000); // default
    wallet.set_fee_rate(20000);
    assert_eq!(wallet.fee_rate(), 20000);
    // Dust threshold should increase with fee rate
    assert!(wallet.dust_threshold() > 546);

    // === Phase: Export state for persistence ===
    let state = wallet.export_state();
    assert_eq!(state.balance, 200_000);
    assert!(state.external_cursor >= 20);
    assert!(state.internal_cursor >= 20);
}

/// WalletManager with persistence: create -> save -> recovery
#[test]
fn test_wallet_manager_persistence_and_recovery() {
    let persistence = Arc::new(InMemoryPersistence::new());

    // === Fresh start ===
    let mut mgr = BitcoinWalletManager::new(test_xpub(), persistence.clone()).unwrap();
    let action = mgr.initialize().unwrap();
    assert!(matches!(action, RecoveryAction::FreshStart));
    assert!(mgr.is_initialized());

    // Add some UTXOs
    mgr.wallet_mut().add_utxo(make_utxo(0x01, 100_000, 800000));
    mgr.wallet_mut().add_utxo(make_utxo(0x02, 200_000, 800001));
    assert_eq!(mgr.balance(), 300_000);

    // Save state
    mgr.save_state().unwrap();

    // Verify state was persisted
    let loaded_wallet = persistence.load_wallet_state().unwrap().unwrap();
    assert_eq!(loaded_wallet.balance, 300_000);

    // === Simulate restart with existing state ===
    let action2 = determine_recovery_action(persistence.as_ref()).unwrap();
    match action2 {
        RecoveryAction::ResumeNormal { wallet_state, .. } => {
            assert_eq!(wallet_state.balance, 300_000);
        },
        other => panic!("expected ResumeNormal, got {:?}", other),
    }

    // === Fee estimation via manager ===
    let fee = mgr.estimate_fee(50_000);
    assert!(fee > 0);

    let max = mgr.max_spendable();
    assert!(max > 0);
    assert!(max < 300_000);

    // === SPV fee rate update ===
    mgr.update_fee_rate_from_peer(15000);
    assert_eq!(mgr.fee_rate(), 15000);
    mgr.update_fee_rate_from_peer(8000); // lower -> no update
    assert_eq!(mgr.fee_rate(), 15000);
}

/// Multi-chain wallet configs are distinct
#[test]
fn test_multi_chain_wallet_configs() {
    use tw_wallet_manager::bcash_wallet_chain::BitcoinCashMainnetWallet;
    use tw_wallet_manager::dogecoin_wallet_chain::DogecoinMainnetWallet;
    use tw_wallet_manager::litecoin_wallet_chain::LitecoinMainnetWallet;
    use tw_wallet_manager::wallet_chain::WalletChain;

    // Unique coin types (SLIP44)
    assert_eq!(BitcoinMainnetWallet::coin_type(), 0);
    assert_eq!(BitcoinCashMainnetWallet::coin_type(), 145);
    assert_eq!(LitecoinMainnetWallet::coin_type(), 2);
    assert_eq!(DogecoinMainnetWallet::coin_type(), 3);

    // Dogecoin has 1 DOGE dust minimum
    let doge_dust = DogecoinMainnetWallet.dust_threshold(10000);
    let btc_dust = BitcoinMainnetWallet.dust_threshold(10000);
    assert!(
        doge_dust > btc_dust,
        "Dogecoin dust ({}) should be higher than Bitcoin ({})",
        doge_dust,
        btc_dust
    );
    assert_eq!(doge_dust, 100_000_000); // 1 DOGE

    // All share same gap limit
    assert_eq!(BitcoinMainnetWallet::external_gap_limit(), 20);
    assert_eq!(BitcoinCashMainnetWallet::external_gap_limit(), 20);
    assert_eq!(LitecoinMainnetWallet::external_gap_limit(), 20);
    assert_eq!(DogecoinMainnetWallet::external_gap_limit(), 20);
}
