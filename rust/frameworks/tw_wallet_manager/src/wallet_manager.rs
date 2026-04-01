// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::bitcoin_wallet_chain::BitcoinMainnetWallet;
use crate::error::WalletResult;
use crate::persistence::{ChainState, Persistence};
use crate::recovery::{determine_recovery_action, RecoveryAction};
use crate::wallet::Wallet;
use bitcoin::bip32::ExtendedPubKey;
use std::sync::Arc;
use tw_hash::H256;

/// Orchestrates a Bitcoin mainnet wallet with persistence and recovery support.
pub struct BitcoinWalletManager {
    wallet: Wallet<BitcoinMainnetWallet>,
    persistence: Arc<dyn Persistence>,
    is_initialized: bool,
}

impl BitcoinWalletManager {
    /// Creates a new wallet manager with the given xpub and persistence backend.
    pub fn new(xpub: ExtendedPubKey, persistence: Arc<dyn Persistence>) -> WalletResult<Self> {
        let wallet = Wallet::new(xpub, BitcoinMainnetWallet)?;
        Ok(Self {
            wallet,
            persistence,
            is_initialized: false,
        })
    }

    /// Initializes the wallet manager by determining the recovery action from persisted state.
    pub fn initialize(&mut self) -> WalletResult<RecoveryAction> {
        let action = determine_recovery_action(self.persistence.as_ref())?;
        self.is_initialized = true;
        Ok(action)
    }

    /// Saves the current wallet state along with chain state to persistence.
    pub fn save_state(&self) -> WalletResult<()> {
        let wallet_state = self.wallet.export_state();
        let chain_state = ChainState {
            last_height: wallet_state.wallet_height,
            last_hash: H256::default(),
            last_timestamp: 0,
        };
        self.persistence
            .save_sync_progress(&chain_state, &wallet_state)?;
        Ok(())
    }

    // --- Delegate methods ---

    /// Returns the total balance.
    pub fn balance(&self) -> u64 {
        self.wallet.balance()
    }

    /// Returns the first unused receiving address.
    pub fn receive_address(&self) -> Option<&str> {
        self.wallet.receive_address()
    }

    /// Returns the first unused change address.
    pub fn change_address(&self) -> Option<&str> {
        self.wallet.change_address()
    }

    /// Returns the number of tracked transactions.
    pub fn transaction_count(&self) -> usize {
        self.wallet.transaction_count()
    }

    /// Returns the number of tracked UTXOs.
    pub fn utxo_count(&self) -> usize {
        self.wallet.utxo_count()
    }

    /// Returns the current fee rate.
    pub fn fee_rate(&self) -> u64 {
        self.wallet.fee_rate()
    }

    /// Sets the fee rate.
    pub fn set_fee_rate(&mut self, fee_rate: u64) {
        self.wallet.set_fee_rate(fee_rate);
    }

    /// Returns a reference to the underlying wallet.
    pub fn wallet(&self) -> &Wallet<BitcoinMainnetWallet> {
        &self.wallet
    }

    /// Returns a mutable reference to the underlying wallet.
    pub fn wallet_mut(&mut self) -> &mut Wallet<BitcoinMainnetWallet> {
        &mut self.wallet
    }

    /// Returns whether the wallet manager has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::test_persistence::InMemoryPersistence;
    use crate::recovery::RecoveryAction;
    use std::str::FromStr;

    fn test_xpub() -> ExtendedPubKey {
        ExtendedPubKey::from_str(
            "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8"
        ).unwrap()
    }

    #[test]
    fn test_wallet_manager_creation() {
        let persistence = Arc::new(InMemoryPersistence::new());
        let mgr = BitcoinWalletManager::new(test_xpub(), persistence).unwrap();
        assert!(!mgr.is_initialized());
        assert_eq!(mgr.balance(), 0);
        assert!(mgr.receive_address().is_some());
    }

    #[test]
    fn test_wallet_manager_initialize_fresh() {
        let persistence = Arc::new(InMemoryPersistence::new());
        let mut mgr = BitcoinWalletManager::new(test_xpub(), persistence).unwrap();
        let action = mgr.initialize().unwrap();
        assert!(matches!(action, RecoveryAction::FreshStart));
        assert!(mgr.is_initialized());
    }

    #[test]
    fn test_wallet_manager_save_and_load() {
        let persistence = Arc::new(InMemoryPersistence::new());
        let mgr = BitcoinWalletManager::new(test_xpub(), persistence.clone()).unwrap();
        mgr.save_state().unwrap();

        // Verify the state was persisted.
        let loaded = persistence.load_wallet_state().unwrap();
        assert!(loaded.is_some());
        let ws = loaded.unwrap();
        assert_eq!(ws.balance, 0);
        assert_eq!(ws.wallet_height, 0);
    }

    #[test]
    fn test_wallet_manager_fee_rate() {
        let persistence = Arc::new(InMemoryPersistence::new());
        let mut mgr = BitcoinWalletManager::new(test_xpub(), persistence).unwrap();
        assert_eq!(mgr.fee_rate(), 10_000);
        mgr.set_fee_rate(25_000);
        assert_eq!(mgr.fee_rate(), 25_000);
    }
}
