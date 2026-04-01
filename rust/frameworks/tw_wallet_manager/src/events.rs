// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::types::Txid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletEvent {
    BalanceChanged {
        new_balance: u64,
    },
    TransactionAdded {
        txid: Txid,
    },
    TransactionUpdated {
        txid: Txid,
        block_height: Option<u32>,
    },
    TransactionDeleted {
        txid: Txid,
        requires_rescan: bool,
    },
    AddressesGenerated {
        external_count: u32,
        internal_count: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tw_hash::H256;

    #[test]
    fn test_balance_changed_equality() {
        let a = WalletEvent::BalanceChanged {
            new_balance: 100_000,
        };
        let b = WalletEvent::BalanceChanged {
            new_balance: 100_000,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_tx_added_match() {
        let txid = H256::default();
        let event = WalletEvent::TransactionAdded { txid };
        match event {
            WalletEvent::TransactionAdded { txid: id } => {
                assert_eq!(id, H256::default());
            },
            _ => panic!("Expected TransactionAdded"),
        }
    }

    #[test]
    fn test_tx_deleted_with_rescan() {
        let txid = H256::default();
        let event = WalletEvent::TransactionDeleted {
            txid,
            requires_rescan: true,
        };
        match event {
            WalletEvent::TransactionDeleted {
                requires_rescan, ..
            } => {
                assert!(requires_rescan);
            },
            _ => panic!("Expected TransactionDeleted"),
        }
    }
}
