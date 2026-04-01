// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::error::PersistenceError;
use crate::types::*;
use std::collections::HashSet;
use tw_hash::H256;
use tw_spv::chain::BlockHeader;
use tw_spv::peer::PeerInfo;

/// Persisted chain synchronization state.
#[derive(Debug, Clone)]
pub struct ChainState {
    pub last_height: u32,
    pub last_hash: H256,
    pub last_timestamp: u32,
}

/// Persisted wallet state including addresses, UTXOs, and transactions.
#[derive(Debug, Clone)]
pub struct WalletState {
    pub external_cursor: u32,
    pub internal_cursor: u32,
    pub used_addresses: HashSet<String>,
    pub utxos: Vec<UtxoRecord>,
    pub transactions: Vec<WalletTransaction>,
    pub balance: u64,
    pub wallet_height: u32,
    pub fee_rate: u64,
}

/// Trait for persisting wallet and chain state.
pub trait Persistence: Send + Sync {
    fn load_peer_cache(&self) -> Result<Vec<PeerInfo>, PersistenceError>;
    fn save_peer_cache(&self, peers: &[PeerInfo]) -> Result<(), PersistenceError>;
    fn load_chain_state(&self) -> Result<Option<ChainState>, PersistenceError>;
    fn save_chain_state(&self, state: &ChainState) -> Result<(), PersistenceError>;
    fn load_wallet_state(&self) -> Result<Option<WalletState>, PersistenceError>;
    fn save_wallet_state(&self, state: &WalletState) -> Result<(), PersistenceError>;
    fn clear_wallet_state(&self) -> Result<(), PersistenceError>;
    fn save_headers_batch(
        &self,
        headers: &[(u32, BlockHeader)],
        chain_state: &ChainState,
    ) -> Result<(), PersistenceError>;
    fn save_sync_progress(
        &self,
        chain_state: &ChainState,
        wallet_state: &WalletState,
    ) -> Result<(), PersistenceError>;
}

#[cfg(test)]
pub mod test_persistence {
    use super::*;
    use std::sync::Mutex;

    /// In-memory implementation of `Persistence` for testing.
    pub struct InMemoryPersistence {
        peers: Mutex<Vec<PeerInfo>>,
        chain_state: Mutex<Option<ChainState>>,
        wallet_state: Mutex<Option<WalletState>>,
        headers: Mutex<Vec<(u32, BlockHeader)>>,
    }

    impl InMemoryPersistence {
        pub fn new() -> Self {
            Self {
                peers: Mutex::new(Vec::new()),
                chain_state: Mutex::new(None),
                wallet_state: Mutex::new(None),
                headers: Mutex::new(Vec::new()),
            }
        }
    }

    impl Persistence for InMemoryPersistence {
        fn load_peer_cache(&self) -> Result<Vec<PeerInfo>, PersistenceError> {
            Ok(self.peers.lock().unwrap().clone())
        }

        fn save_peer_cache(&self, peers: &[PeerInfo]) -> Result<(), PersistenceError> {
            *self.peers.lock().unwrap() = peers.to_vec();
            Ok(())
        }

        fn load_chain_state(&self) -> Result<Option<ChainState>, PersistenceError> {
            Ok(self.chain_state.lock().unwrap().clone())
        }

        fn save_chain_state(&self, state: &ChainState) -> Result<(), PersistenceError> {
            *self.chain_state.lock().unwrap() = Some(state.clone());
            Ok(())
        }

        fn load_wallet_state(&self) -> Result<Option<WalletState>, PersistenceError> {
            Ok(self.wallet_state.lock().unwrap().clone())
        }

        fn save_wallet_state(&self, state: &WalletState) -> Result<(), PersistenceError> {
            *self.wallet_state.lock().unwrap() = Some(state.clone());
            Ok(())
        }

        fn clear_wallet_state(&self) -> Result<(), PersistenceError> {
            *self.wallet_state.lock().unwrap() = None;
            Ok(())
        }

        fn save_headers_batch(
            &self,
            headers: &[(u32, BlockHeader)],
            chain_state: &ChainState,
        ) -> Result<(), PersistenceError> {
            self.headers.lock().unwrap().extend_from_slice(headers);
            *self.chain_state.lock().unwrap() = Some(chain_state.clone());
            Ok(())
        }

        fn save_sync_progress(
            &self,
            chain_state: &ChainState,
            wallet_state: &WalletState,
        ) -> Result<(), PersistenceError> {
            *self.chain_state.lock().unwrap() = Some(chain_state.clone());
            *self.wallet_state.lock().unwrap() = Some(wallet_state.clone());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_persistence::InMemoryPersistence;
    use super::*;

    fn sample_chain_state() -> ChainState {
        ChainState {
            last_height: 800_000,
            last_hash: H256::default(),
            last_timestamp: 1_700_000_000,
        }
    }

    fn sample_wallet_state(wallet_height: u32) -> WalletState {
        WalletState {
            external_cursor: 5,
            internal_cursor: 2,
            used_addresses: HashSet::from(["addr1".to_string(), "addr2".to_string()]),
            utxos: vec![],
            transactions: vec![],
            balance: 100_000,
            wallet_height,
            fee_rate: 10,
        }
    }

    #[test]
    fn test_in_memory_persistence_roundtrip() {
        let store = InMemoryPersistence::new();

        // Save chain and wallet state.
        let chain = sample_chain_state();
        let wallet = sample_wallet_state(800_000);
        store.save_chain_state(&chain).unwrap();
        store.save_wallet_state(&wallet).unwrap();

        // Load and verify.
        let loaded_chain = store.load_chain_state().unwrap().expect("chain state");
        assert_eq!(loaded_chain.last_height, 800_000);
        assert_eq!(loaded_chain.last_timestamp, 1_700_000_000);

        let loaded_wallet = store.load_wallet_state().unwrap().expect("wallet state");
        assert_eq!(loaded_wallet.external_cursor, 5);
        assert_eq!(loaded_wallet.balance, 100_000);

        // Clear wallet state — chain state should remain.
        store.clear_wallet_state().unwrap();
        assert!(store.load_wallet_state().unwrap().is_none());
        assert!(store.load_chain_state().unwrap().is_some());
    }

    #[test]
    fn test_atomic_save_sync_progress() {
        let store = InMemoryPersistence::new();

        let chain = sample_chain_state();
        let wallet = sample_wallet_state(800_000);
        store.save_sync_progress(&chain, &wallet).unwrap();

        let loaded_chain = store.load_chain_state().unwrap().expect("chain state");
        assert_eq!(loaded_chain.last_height, 800_000);

        let loaded_wallet = store.load_wallet_state().unwrap().expect("wallet state");
        assert_eq!(loaded_wallet.wallet_height, 800_000);
        assert_eq!(loaded_wallet.balance, 100_000);
    }
}
