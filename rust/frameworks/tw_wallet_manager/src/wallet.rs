// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::address_manager::AddressManager;
use crate::error::WalletResult;
use crate::events::WalletEvent;
use crate::persistence::WalletState;
use crate::tx_tracker::TxTracker;
use crate::types::{Txid, UtxoRecord, WalletTransaction};
use crate::utxo_tracker::UtxoTracker;
use crate::wallet_chain::WalletChain;
use bitcoin::bip32::ExtendedPubKey;
use std::collections::HashSet;
use tw_utxo::transaction::transaction_parts::OutPoint;

/// Facade integrating address management, UTXO tracking, and transaction tracking
/// into a single cohesive wallet interface.
pub struct Wallet<C: WalletChain> {
    chain: C,
    addresses: AddressManager,
    utxos: UtxoTracker,
    txs: TxTracker,
    fee_rate: u64,
    wallet_height: u32,
}

impl<C: WalletChain> Wallet<C> {
    /// Create a new wallet from an extended public key and chain policy.
    pub fn new(xpub: ExtendedPubKey, chain: C) -> WalletResult<Self> {
        let addresses = AddressManager::new(xpub, &chain)?;
        Ok(Self {
            chain,
            addresses,
            utxos: UtxoTracker::new(),
            txs: TxTracker::new(),
            fee_rate: 10_000,
            wallet_height: 0,
        })
    }

    // --- Balance methods ---

    /// Returns the total balance (confirmed + unconfirmed).
    pub fn balance(&self) -> u64 {
        self.utxos.balance()
    }

    /// Returns only the confirmed balance.
    pub fn confirmed_balance(&self) -> u64 {
        self.utxos.confirmed_balance()
    }

    /// Returns the total amount received across all transactions.
    pub fn total_received(&self) -> u64 {
        self.txs.total_received()
    }

    /// Returns the total amount sent across all transactions.
    pub fn total_sent(&self) -> u64 {
        self.txs.total_sent()
    }

    // --- Fee rate methods ---

    /// Returns the current fee rate.
    pub fn fee_rate(&self) -> u64 {
        self.fee_rate
    }

    /// Sets the fee rate.
    pub fn set_fee_rate(&mut self, fee_rate: u64) {
        self.fee_rate = fee_rate;
    }

    /// Returns the dust threshold at the current fee rate.
    pub fn dust_threshold(&self) -> u64 {
        self.chain.dust_threshold(self.fee_rate)
    }

    // --- Address methods ---

    /// Returns the first unused external (receiving) address.
    pub fn receive_address(&self) -> Option<&str> {
        self.addresses.receive_address()
    }

    /// Returns the first unused internal (change) address.
    pub fn change_address(&self) -> Option<&str> {
        self.addresses.change_address()
    }

    /// Returns whether the wallet contains the given address.
    pub fn contains_address(&self, address: &str) -> bool {
        self.addresses.contains_address(address)
    }

    /// Returns all addresses known to the wallet.
    pub fn all_addresses(&self) -> HashSet<String> {
        self.addresses.all_addresses().clone()
    }

    // --- UTXO methods ---

    /// Adds a UTXO and emits a BalanceChanged event if the balance changed.
    pub fn add_utxo(&mut self, utxo: UtxoRecord) -> Vec<WalletEvent> {
        let old_balance = self.utxos.balance();
        self.utxos.add(utxo);
        let new_balance = self.utxos.balance();

        let mut events = Vec::new();
        if new_balance != old_balance {
            events.push(WalletEvent::BalanceChanged { new_balance });
        }
        events
    }

    /// Spends (removes) a UTXO and emits a BalanceChanged event if the balance changed.
    pub fn spend_utxo(&mut self, outpoint: &OutPoint) -> Vec<WalletEvent> {
        let old_balance = self.utxos.balance();
        self.utxos.remove(outpoint);
        let new_balance = self.utxos.balance();

        let mut events = Vec::new();
        if new_balance != old_balance {
            events.push(WalletEvent::BalanceChanged { new_balance });
        }
        events
    }

    // --- Transaction methods ---

    /// Registers a transaction and emits TransactionAdded if it is new.
    pub fn register_transaction(&mut self, tx: WalletTransaction) -> Vec<WalletEvent> {
        let txid = tx.txid;
        let mut events = Vec::new();
        if self.txs.register(tx) {
            events.push(WalletEvent::TransactionAdded { txid });
        }
        events
    }

    // --- Address used ---

    /// Marks an address as used, ensures gap limit, and emits AddressesGenerated if new
    /// addresses were derived.
    pub fn mark_address_used(&mut self, address: &str) -> WalletResult<Vec<WalletEvent>> {
        let mut events = Vec::new();
        if self.addresses.mark_address_used(address) {
            let (ext_new, int_new) = self.addresses.ensure_gap_limit()?;
            if ext_new > 0 || int_new > 0 {
                events.push(WalletEvent::AddressesGenerated {
                    external_count: ext_new,
                    internal_count: int_new,
                });
            }
        }
        Ok(events)
    }

    // --- Reorg handling ---

    /// Handles a chain reorganization at the given height.
    ///
    /// Unconfirms transactions after the height, removes UTXOs after the height,
    /// and emits appropriate events.
    pub fn handle_reorg(&mut self, height: u32) -> Vec<WalletEvent> {
        let mut events = Vec::new();

        // Unconfirm transactions above the reorg height.
        let unconfirmed_txids = self.txs.set_unconfirmed_after(height);
        for txid in unconfirmed_txids {
            events.push(WalletEvent::TransactionUpdated {
                txid,
                block_height: None,
            });
        }

        // Remove UTXOs confirmed after the reorg height.
        let removed_utxos = self.utxos.remove_after_height(height);
        if !removed_utxos.is_empty() {
            events.push(WalletEvent::BalanceChanged {
                new_balance: self.utxos.balance(),
            });
        }

        events
    }

    // --- Persistence ---

    /// Exports the wallet state for persistence.
    pub fn export_state(&self) -> WalletState {
        let (ext_cursor, int_cursor) = self.addresses.cursors();
        WalletState {
            external_cursor: ext_cursor,
            internal_cursor: int_cursor,
            used_addresses: self.addresses.used_addresses().clone(),
            utxos: self.utxos.export(),
            transactions: self.txs.export(),
            balance: self.utxos.balance(),
            wallet_height: self.wallet_height,
            fee_rate: self.fee_rate,
        }
    }

    // --- Query methods ---

    /// Returns a reference to the transaction with the given txid.
    pub fn get_transaction(&self, txid: &Txid) -> Option<&WalletTransaction> {
        self.txs.get(txid)
    }

    /// Returns all transactions sorted by timestamp descending.
    pub fn transactions(&self) -> Vec<&WalletTransaction> {
        self.txs.all_sorted()
    }

    /// Returns the number of tracked UTXOs.
    pub fn utxo_count(&self) -> usize {
        self.utxos.count()
    }

    /// Returns the number of tracked transactions.
    pub fn transaction_count(&self) -> usize {
        self.txs.count()
    }

    /// Returns the current wallet height.
    pub fn wallet_height(&self) -> u32 {
        self.wallet_height
    }

    /// Sets the wallet height.
    pub fn set_wallet_height(&mut self, height: u32) {
        self.wallet_height = height;
    }

    /// Clears all UTXOs, transactions, and resets wallet height for a rescan.
    pub fn clear_for_rescan(&mut self) {
        self.utxos.clear();
        self.txs.clear();
        self.wallet_height = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin_wallet_chain::BitcoinMainnetWallet;
    use std::str::FromStr;
    use tw_hash::H256;

    fn test_xpub() -> ExtendedPubKey {
        ExtendedPubKey::from_str(
            "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8"
        ).unwrap()
    }

    fn make_utxo(index: u32, value: u64, block_height: Option<u32>) -> UtxoRecord {
        UtxoRecord {
            outpoint: OutPoint {
                hash: H256::default(),
                index,
            },
            value,
            script: vec![],
            block_height,
            is_change: false,
        }
    }

    fn make_tx(
        txid_byte: u8,
        received: u64,
        sent: u64,
        block_height: Option<u32>,
    ) -> WalletTransaction {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = txid_byte;
        WalletTransaction {
            txid: H256::from(hash_bytes),
            raw_tx: vec![],
            block_height,
            timestamp: 0,
            amount_received: received,
            amount_sent: sent,
            fee: None,
        }
    }

    #[test]
    fn test_wallet_initial_state() {
        let wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();
        assert_eq!(wallet.balance(), 0);
        assert!(wallet.receive_address().is_some());
        assert!(wallet.change_address().is_some());
    }

    #[test]
    fn test_wallet_add_utxo_emits_balance() {
        let mut wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();
        let events = wallet.add_utxo(make_utxo(0, 50_000, Some(100)));
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            WalletEvent::BalanceChanged {
                new_balance: 50_000
            }
        );
        assert_eq!(wallet.balance(), 50_000);
    }

    #[test]
    fn test_wallet_spend_utxo() {
        let mut wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();
        let utxo = make_utxo(0, 50_000, Some(100));
        let outpoint = utxo.outpoint;
        wallet.add_utxo(utxo);
        assert_eq!(wallet.balance(), 50_000);

        let events = wallet.spend_utxo(&outpoint);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], WalletEvent::BalanceChanged { new_balance: 0 });
        assert_eq!(wallet.balance(), 0);
    }

    #[test]
    fn test_wallet_register_transaction() {
        let mut wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();
        let tx = make_tx(1, 50_000, 0, Some(100));
        let txid = tx.txid;

        let events = wallet.register_transaction(tx);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], WalletEvent::TransactionAdded { txid });
    }

    #[test]
    fn test_wallet_reorg() {
        let mut wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();
        // Add UTXOs at different heights.
        wallet.add_utxo(make_utxo(0, 10_000, Some(100)));
        wallet.add_utxo(make_utxo(1, 20_000, Some(200)));
        assert_eq!(wallet.balance(), 30_000);

        // Reorg at height 150 should remove the UTXO at height 200.
        let events = wallet.handle_reorg(150);
        assert_eq!(wallet.balance(), 10_000);
        // Should have a BalanceChanged event.
        assert!(events.iter().any(|e| matches!(
            e,
            WalletEvent::BalanceChanged {
                new_balance: 10_000
            }
        )));
    }

    #[test]
    fn test_wallet_export_state() {
        let mut wallet = Wallet::new(test_xpub(), BitcoinMainnetWallet).unwrap();
        wallet.set_wallet_height(500);
        wallet.set_fee_rate(20_000);
        wallet.add_utxo(make_utxo(0, 75_000, Some(500)));

        let state = wallet.export_state();
        assert_eq!(state.balance, 75_000);
        assert_eq!(state.wallet_height, 500);
        assert_eq!(state.fee_rate, 20_000);
        assert!(state.external_cursor > 0);
        assert!(state.internal_cursor > 0);
    }
}
