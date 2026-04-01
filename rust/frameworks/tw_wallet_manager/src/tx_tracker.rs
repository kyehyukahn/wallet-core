// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::types::{Txid, WalletTransaction};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct TxTracker {
    transactions: HashMap<Txid, WalletTransaction>,
}

impl TxTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a transaction. Returns `true` if the transaction is new.
    pub fn register(&mut self, tx: WalletTransaction) -> bool {
        use std::collections::hash_map::Entry;
        match self.transactions.entry(tx.txid) {
            Entry::Vacant(e) => {
                e.insert(tx);
                true
            },
            Entry::Occupied(_) => false,
        }
    }

    /// Removes a transaction by txid. Returns the removed transaction if it existed.
    pub fn remove(&mut self, txid: &Txid) -> Option<WalletTransaction> {
        self.transactions.remove(txid)
    }

    /// Returns a reference to the transaction with the given txid.
    pub fn get(&self, txid: &Txid) -> Option<&WalletTransaction> {
        self.transactions.get(txid)
    }

    /// Returns whether a transaction with the given txid exists.
    pub fn contains(&self, txid: &Txid) -> bool {
        self.transactions.contains_key(txid)
    }

    /// Updates confirmations for the given txids by setting their block height and timestamp.
    /// Returns the list of txids that were actually updated.
    pub fn update_confirmations(
        &mut self,
        txids: &[Txid],
        block_height: u32,
        timestamp: u32,
    ) -> Vec<Txid> {
        let mut updated = Vec::new();
        for txid in txids {
            if let Some(tx) = self.transactions.get_mut(txid) {
                tx.block_height = Some(block_height);
                tx.timestamp = timestamp;
                updated.push(*txid);
            }
        }
        updated
    }

    /// Handles a reorg: unconfirms all transactions with block_height > `height`
    /// by setting their block_height to `None` and timestamp to 0.
    /// Returns the list of txids that were unconfirmed.
    pub fn set_unconfirmed_after(&mut self, height: u32) -> Vec<Txid> {
        let mut affected = Vec::new();
        for (txid, tx) in self.transactions.iter_mut() {
            if let Some(h) = tx.block_height {
                if h > height {
                    tx.block_height = None;
                    tx.timestamp = 0;
                    affected.push(*txid);
                }
            }
        }
        affected
    }

    /// Returns all transactions sorted by timestamp descending.
    pub fn all_sorted(&self) -> Vec<&WalletTransaction> {
        let mut txs: Vec<&WalletTransaction> = self.transactions.values().collect();
        txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        txs
    }

    /// Returns only unconfirmed transactions.
    pub fn unconfirmed(&self) -> Vec<&WalletTransaction> {
        self.transactions
            .values()
            .filter(|tx| !tx.is_confirmed())
            .collect()
    }

    /// Returns the total amount received across all transactions.
    pub fn total_received(&self) -> u64 {
        self.transactions
            .values()
            .map(|tx| tx.amount_received)
            .sum()
    }

    /// Returns the total amount sent across all transactions.
    pub fn total_sent(&self) -> u64 {
        self.transactions.values().map(|tx| tx.amount_sent).sum()
    }

    /// Returns the number of tracked transactions.
    pub fn count(&self) -> usize {
        self.transactions.len()
    }

    /// Clears all tracked transactions.
    pub fn clear(&mut self) {
        self.transactions.clear();
    }

    /// Loads transactions from a vector, replacing all existing transactions.
    pub fn load(&mut self, txs: Vec<WalletTransaction>) {
        self.transactions.clear();
        for tx in txs {
            self.transactions.insert(tx.txid, tx);
        }
    }

    /// Exports all transactions as a vector.
    pub fn export(&self) -> Vec<WalletTransaction> {
        self.transactions.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tw_hash::H256;

    fn make_tx(
        txid_byte: u8,
        received: u64,
        sent: u64,
        block_height: Option<u32>,
        timestamp: u32,
    ) -> WalletTransaction {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = txid_byte;
        WalletTransaction {
            txid: H256::from(hash_bytes),
            raw_tx: vec![],
            block_height,
            timestamp,
            amount_received: received,
            amount_sent: sent,
            fee: None,
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut tracker = TxTracker::new();
        let tx = make_tx(1, 50_000, 0, None, 0);
        let txid = tx.txid;

        assert!(tracker.register(tx.clone()));
        // Duplicate returns false.
        assert!(!tracker.register(tx));
        assert_eq!(tracker.count(), 1);
        assert!(tracker.contains(&txid));
        assert!(tracker.get(&txid).is_some());
    }

    #[test]
    fn test_remove_transaction() {
        let mut tracker = TxTracker::new();
        let tx = make_tx(2, 10_000, 0, Some(100), 1000);
        let txid = tx.txid;

        tracker.register(tx);
        assert_eq!(tracker.count(), 1);

        let removed = tracker.remove(&txid);
        assert!(removed.is_some());
        assert_eq!(tracker.count(), 0);
        assert!(!tracker.contains(&txid));
    }

    #[test]
    fn test_update_confirmations() {
        let mut tracker = TxTracker::new();
        let tx = make_tx(3, 25_000, 0, None, 0);
        let txid = tx.txid;

        tracker.register(tx);
        assert!(!tracker.get(&txid).unwrap().is_confirmed());

        let updated = tracker.update_confirmations(&[txid], 500, 1_600_000_000);
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0], txid);

        let confirmed_tx = tracker.get(&txid).unwrap();
        assert!(confirmed_tx.is_confirmed());
        assert_eq!(confirmed_tx.block_height, Some(500));
        assert_eq!(confirmed_tx.timestamp, 1_600_000_000);
    }

    #[test]
    fn test_set_unconfirmed_after_reorg() {
        let mut tracker = TxTracker::new();
        let tx1 = make_tx(10, 1000, 0, Some(100), 1000);
        let tx2 = make_tx(20, 2000, 0, Some(200), 2000);
        let tx3 = make_tx(30, 3000, 0, Some(300), 3000);
        let txid1 = tx1.txid;
        let txid2 = tx2.txid;
        let txid3 = tx3.txid;

        tracker.register(tx1);
        tracker.register(tx2);
        tracker.register(tx3);

        let affected = tracker.set_unconfirmed_after(150);
        // tx2 (height 200) and tx3 (height 300) should be unconfirmed.
        assert_eq!(affected.len(), 2);

        // tx1 at height 100 remains confirmed.
        let t1 = tracker.get(&txid1).unwrap();
        assert!(t1.is_confirmed());
        assert_eq!(t1.timestamp, 1000);

        // tx2 and tx3 should be unconfirmed with timestamp 0.
        let t2 = tracker.get(&txid2).unwrap();
        assert!(!t2.is_confirmed());
        assert_eq!(t2.timestamp, 0);

        let t3 = tracker.get(&txid3).unwrap();
        assert!(!t3.is_confirmed());
        assert_eq!(t3.timestamp, 0);
    }

    #[test]
    fn test_totals() {
        let mut tracker = TxTracker::new();
        tracker.register(make_tx(1, 50_000, 10_000, Some(100), 1000));
        tracker.register(make_tx(2, 30_000, 20_000, Some(200), 2000));

        assert_eq!(tracker.total_received(), 80_000);
        assert_eq!(tracker.total_sent(), 30_000);
    }

    #[test]
    fn test_unconfirmed_filter() {
        let mut tracker = TxTracker::new();
        tracker.register(make_tx(1, 10_000, 0, Some(100), 1000));
        tracker.register(make_tx(2, 20_000, 0, None, 0));

        let unconfirmed = tracker.unconfirmed();
        assert_eq!(unconfirmed.len(), 1);
        assert!(!unconfirmed[0].is_confirmed());
    }
}
