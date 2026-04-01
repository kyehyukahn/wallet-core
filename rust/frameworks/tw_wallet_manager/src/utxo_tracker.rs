// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::types::UtxoRecord;
use std::collections::HashMap;
use tw_utxo::transaction::transaction_parts::OutPoint;

/// Tracks unspent transaction outputs (UTXOs) and provides balance calculation,
/// policy-aware sorting at query time, and reorg support.
pub struct UtxoTracker {
    utxos: HashMap<OutPoint, UtxoRecord>,
}

impl Default for UtxoTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UtxoTracker {
    /// Creates a new empty UTXO tracker.
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }

    /// Adds a UTXO record to the tracker.
    pub fn add(&mut self, utxo: UtxoRecord) {
        self.utxos.insert(utxo.outpoint, utxo);
    }

    /// Removes a UTXO by its outpoint, returning it if it existed.
    pub fn remove(&mut self, outpoint: &OutPoint) -> Option<UtxoRecord> {
        self.utxos.remove(outpoint)
    }

    /// Returns whether the tracker contains the given outpoint.
    pub fn contains(&self, outpoint: &OutPoint) -> bool {
        self.utxos.contains_key(outpoint)
    }

    /// Returns a reference to the UTXO record for the given outpoint.
    pub fn get(&self, outpoint: &OutPoint) -> Option<&UtxoRecord> {
        self.utxos.get(outpoint)
    }

    /// Returns the total balance (sum of all UTXO values).
    pub fn balance(&self) -> u64 {
        self.utxos.values().map(|u| u.value).sum()
    }

    /// Returns the confirmed balance (sum of values where block_height is Some).
    pub fn confirmed_balance(&self) -> u64 {
        self.utxos
            .values()
            .filter(|u| u.block_height.is_some())
            .map(|u| u.value)
            .sum()
    }

    /// Returns the number of tracked UTXOs.
    pub fn count(&self) -> usize {
        self.utxos.len()
    }

    /// Returns an iterator over all UTXO records.
    pub fn all(&self) -> impl Iterator<Item = &UtxoRecord> {
        self.utxos.values()
    }

    /// Returns UTXOs sorted by value in ascending order.
    pub fn sorted_by_value_asc(&self) -> Vec<&UtxoRecord> {
        let mut records: Vec<&UtxoRecord> = self.utxos.values().collect();
        records.sort_by_key(|u| u.value);
        records
    }

    /// Returns UTXOs sorted by value in descending order.
    pub fn sorted_by_value_desc(&self) -> Vec<&UtxoRecord> {
        let mut records: Vec<&UtxoRecord> = self.utxos.values().collect();
        records.sort_by(|a, b| b.value.cmp(&a.value));
        records
    }

    /// Removes all tracked UTXOs.
    pub fn clear(&mut self) {
        self.utxos.clear();
    }

    /// Bulk-loads UTXOs, replacing any existing entries.
    pub fn load(&mut self, utxos: Vec<UtxoRecord>) {
        self.utxos.clear();
        for utxo in utxos {
            self.utxos.insert(utxo.outpoint, utxo);
        }
    }

    /// Exports all tracked UTXOs as a Vec.
    pub fn export(&self) -> Vec<UtxoRecord> {
        self.utxos.values().cloned().collect()
    }

    /// Removes UTXOs confirmed after the given height (for reorg handling).
    /// Returns the removed UTXOs.
    pub fn remove_after_height(&mut self, height: u32) -> Vec<UtxoRecord> {
        let to_remove: Vec<OutPoint> = self
            .utxos
            .values()
            .filter(|u| matches!(u.block_height, Some(h) if h > height))
            .map(|u| u.outpoint)
            .collect();

        to_remove
            .into_iter()
            .filter_map(|op| self.utxos.remove(&op))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tw_hash::H256;

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

    #[test]
    fn test_add_and_balance() {
        let mut tracker = UtxoTracker::new();
        tracker.add(make_utxo(0, 50_000, Some(100)));
        tracker.add(make_utxo(1, 30_000, Some(101)));
        assert_eq!(tracker.balance(), 80_000);
        assert_eq!(tracker.count(), 2);
    }

    #[test]
    fn test_confirmed_vs_unconfirmed_balance() {
        let mut tracker = UtxoTracker::new();
        tracker.add(make_utxo(0, 50_000, Some(100)));
        tracker.add(make_utxo(1, 30_000, None));
        assert_eq!(tracker.balance(), 80_000);
        assert_eq!(tracker.confirmed_balance(), 50_000);
    }

    #[test]
    fn test_remove_utxo() {
        let mut tracker = UtxoTracker::new();
        let utxo = make_utxo(0, 50_000, Some(100));
        let outpoint = utxo.outpoint;
        tracker.add(utxo);
        assert_eq!(tracker.balance(), 50_000);

        let removed = tracker.remove(&outpoint);
        assert!(removed.is_some());
        assert_eq!(tracker.balance(), 0);
    }

    #[test]
    fn test_sorted_by_value() {
        let mut tracker = UtxoTracker::new();
        tracker.add(make_utxo(0, 30_000, Some(100)));
        tracker.add(make_utxo(1, 10_000, Some(101)));
        tracker.add(make_utxo(2, 50_000, Some(102)));

        let asc = tracker.sorted_by_value_asc();
        assert_eq!(asc[0].value, 10_000);
        assert_eq!(asc[1].value, 30_000);
        assert_eq!(asc[2].value, 50_000);

        let desc = tracker.sorted_by_value_desc();
        assert_eq!(desc[0].value, 50_000);
        assert_eq!(desc[1].value, 30_000);
        assert_eq!(desc[2].value, 10_000);
    }

    #[test]
    fn test_remove_after_height() {
        let mut tracker = UtxoTracker::new();
        tracker.add(make_utxo(0, 10_000, Some(100)));
        tracker.add(make_utxo(1, 20_000, Some(200)));
        tracker.add(make_utxo(2, 30_000, Some(300)));

        let removed = tracker.remove_after_height(150);
        assert_eq!(removed.len(), 2);
        assert_eq!(tracker.count(), 1);
        assert_eq!(tracker.balance(), 10_000);
    }

    #[test]
    fn test_clear_and_load() {
        let mut tracker = UtxoTracker::new();
        tracker.add(make_utxo(0, 10_000, Some(100)));
        assert_eq!(tracker.count(), 1);

        tracker.clear();
        assert_eq!(tracker.count(), 0);
        assert_eq!(tracker.balance(), 0);

        let utxos = vec![
            make_utxo(0, 25_000, Some(200)),
            make_utxo(1, 35_000, Some(201)),
        ];
        tracker.load(utxos);
        assert_eq!(tracker.count(), 2);
        assert_eq!(tracker.balance(), 60_000);
    }
}
