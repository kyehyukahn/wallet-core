// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use tw_hash::H256;
use tw_utxo::transaction::transaction_parts::OutPoint;

pub type Txid = H256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoRecord {
    pub outpoint: OutPoint,
    pub value: u64,
    pub script: Vec<u8>,
    pub block_height: Option<u32>,
    pub is_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletTransaction {
    pub txid: Txid,
    pub raw_tx: Vec<u8>,
    pub block_height: Option<u32>,
    pub timestamp: u32,
    pub amount_received: u64,
    pub amount_sent: u64,
    pub fee: Option<u64>,
}

impl WalletTransaction {
    pub fn net_amount(&self) -> i64 {
        self.amount_received as i64 - self.amount_sent as i64
    }

    pub fn is_confirmed(&self) -> bool {
        self.block_height.is_some()
    }

    pub fn confirmations(&self, current_height: u32) -> u32 {
        match self.block_height {
            Some(height) => current_height.saturating_sub(height) + 1,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tx(received: u64, sent: u64, block_height: Option<u32>) -> WalletTransaction {
        WalletTransaction {
            txid: H256::default(),
            raw_tx: vec![],
            block_height,
            timestamp: 0,
            amount_received: received,
            amount_sent: sent,
            fee: None,
        }
    }

    #[test]
    fn test_net_amount_received() {
        let tx = sample_tx(50_000, 10_000, Some(100));
        assert_eq!(tx.net_amount(), 40_000);
    }

    #[test]
    fn test_net_amount_sent() {
        let tx = sample_tx(10_000, 50_000, Some(100));
        assert_eq!(tx.net_amount(), -40_000);
    }

    #[test]
    fn test_confirmations_confirmed() {
        let tx = sample_tx(0, 0, Some(800_000));
        assert_eq!(tx.confirmations(800_005), 6);
    }

    #[test]
    fn test_confirmations_unconfirmed() {
        let tx = sample_tx(0, 0, None);
        assert_eq!(tx.confirmations(800_005), 0);
    }

    #[test]
    fn test_is_confirmed() {
        let confirmed = sample_tx(0, 0, Some(100));
        assert!(confirmed.is_confirmed());

        let unconfirmed = sample_tx(0, 0, None);
        assert!(!unconfirmed.is_confirmed());
    }

    #[test]
    fn test_utxo_record_fields() {
        let record = UtxoRecord {
            outpoint: OutPoint {
                hash: H256::default(),
                index: 0,
            },
            value: 100_000,
            script: vec![0x76, 0xa9],
            block_height: Some(500_000),
            is_change: true,
        };
        assert_eq!(record.value, 100_000);
        assert_eq!(record.script, vec![0x76, 0xa9]);
        assert_eq!(record.block_height, Some(500_000));
        assert!(record.is_change);
    }
}
