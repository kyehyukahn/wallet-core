// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::error::WalletResult;
use crate::persistence::{ChainState, Persistence, WalletState};
use tw_spv::peer::PeerInfo;

/// Describes the action to take when recovering wallet state at startup.
#[derive(Debug)]
pub enum RecoveryAction {
    /// Both chain and wallet state are intact and consistent.
    ResumeNormal {
        chain_state: ChainState,
        wallet_state: WalletState,
        peers: Vec<PeerInfo>,
    },
    /// Wallet state needs rescanning (missing or inconsistent).
    WalletRescan {
        chain_state: ChainState,
        peers: Vec<PeerInfo>,
    },
    /// Chain state is missing; need full resync but have peer cache.
    FullResync { peers: Vec<PeerInfo> },
    /// No persisted state at all.
    FreshStart,
}

/// Determines the appropriate recovery action based on persisted state.
pub fn determine_recovery_action(persistence: &dyn Persistence) -> WalletResult<RecoveryAction> {
    let chain_state = persistence.load_chain_state()?;
    let wallet_state = persistence.load_wallet_state()?;
    let peers = persistence.load_peer_cache()?;

    match (chain_state, wallet_state) {
        (Some(cs), Some(ws)) => {
            if ws.wallet_height <= cs.last_height {
                Ok(RecoveryAction::ResumeNormal {
                    chain_state: cs,
                    wallet_state: ws,
                    peers,
                })
            } else {
                // Wallet height exceeds chain height — inconsistent state.
                Ok(RecoveryAction::WalletRescan {
                    chain_state: cs,
                    peers,
                })
            }
        },
        (Some(cs), None) => Ok(RecoveryAction::WalletRescan {
            chain_state: cs,
            peers,
        }),
        (None, _) => {
            if !peers.is_empty() {
                Ok(RecoveryAction::FullResync { peers })
            } else {
                Ok(RecoveryAction::FreshStart)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::test_persistence::InMemoryPersistence;
    use std::collections::HashSet;
    use tw_hash::H256;

    fn sample_chain_state(height: u32) -> ChainState {
        ChainState {
            last_height: height,
            last_hash: H256::default(),
            last_timestamp: 1_700_000_000,
        }
    }

    fn sample_wallet_state(wallet_height: u32) -> crate::persistence::WalletState {
        crate::persistence::WalletState {
            external_cursor: 5,
            internal_cursor: 2,
            used_addresses: HashSet::new(),
            utxos: vec![],
            transactions: vec![],
            balance: 50_000,
            wallet_height,
            fee_rate: 10,
        }
    }

    #[test]
    fn test_fresh_start_no_state() {
        let store = InMemoryPersistence::new();
        let action = determine_recovery_action(&store).unwrap();
        assert!(matches!(action, RecoveryAction::FreshStart));
    }

    #[test]
    fn test_resume_normal() {
        let store = InMemoryPersistence::new();
        let chain = sample_chain_state(800_000);
        let wallet = sample_wallet_state(800_000);
        store.save_chain_state(&chain).unwrap();
        store.save_wallet_state(&wallet).unwrap();

        let action = determine_recovery_action(&store).unwrap();
        match action {
            RecoveryAction::ResumeNormal {
                chain_state,
                wallet_state,
                ..
            } => {
                assert_eq!(chain_state.last_height, 800_000);
                assert_eq!(wallet_state.wallet_height, 800_000);
            },
            other => panic!("Expected ResumeNormal, got {other:?}"),
        }
    }

    #[test]
    fn test_wallet_rescan_missing_wallet() {
        let store = InMemoryPersistence::new();
        let chain = sample_chain_state(800_000);
        store.save_chain_state(&chain).unwrap();

        let action = determine_recovery_action(&store).unwrap();
        match action {
            RecoveryAction::WalletRescan { chain_state, .. } => {
                assert_eq!(chain_state.last_height, 800_000);
            },
            other => panic!("Expected WalletRescan, got {other:?}"),
        }
    }

    #[test]
    fn test_wallet_rescan_inconsistent_height() {
        let store = InMemoryPersistence::new();
        let chain = sample_chain_state(800_000);
        let wallet = sample_wallet_state(900_000); // wallet_height > chain_height
        store.save_chain_state(&chain).unwrap();
        store.save_wallet_state(&wallet).unwrap();

        let action = determine_recovery_action(&store).unwrap();
        match action {
            RecoveryAction::WalletRescan { chain_state, .. } => {
                assert_eq!(chain_state.last_height, 800_000);
            },
            other => panic!("Expected WalletRescan, got {other:?}"),
        }
    }
}
