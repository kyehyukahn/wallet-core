// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncCapability {
    FullBip37,
    /// Reserved for future BIP157/158 support (design spec §9-5).
    CompactFilters,
    HeaderOnly,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    PeerConnected(SocketAddr),
    PeerDisconnected(SocketAddr),
    SyncStarted {
        start_height: u32,
    },
    HeadersProgress {
        current_height: u32,
        target_height: u32,
    },
    FilteredBlockApplied {
        height: u32,
        matched_tx_count: usize,
    },
    CapabilityChanged(SyncCapability),
    SyncCompleted {
        height: u32,
    },
    SyncFailed(SyncError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    NoBip37Peers,
    MaxConnectionFailures { count: u32 },
    NetworkUnreachable,
    PersistenceError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_capability_equality() {
        assert_eq!(SyncCapability::FullBip37, SyncCapability::FullBip37);
        assert_ne!(SyncCapability::FullBip37, SyncCapability::CompactFilters);
        assert_ne!(SyncCapability::HeaderOnly, SyncCapability::Unsupported);
    }

    #[test]
    fn test_sync_capability_copy() {
        let cap = SyncCapability::CompactFilters;
        let cap2 = cap;
        assert_eq!(cap, cap2);
    }

    #[test]
    fn test_sync_event_peer_connected() {
        let addr: SocketAddr = "127.0.0.1:8333".parse().unwrap();
        let event = SyncEvent::PeerConnected(addr);
        assert_eq!(event, SyncEvent::PeerConnected(addr));
    }

    #[test]
    fn test_sync_event_peer_disconnected() {
        let addr: SocketAddr = "192.168.1.1:8333".parse().unwrap();
        let event = SyncEvent::PeerDisconnected(addr);
        assert_eq!(event, SyncEvent::PeerDisconnected(addr));
    }

    #[test]
    fn test_sync_event_sync_started() {
        let event = SyncEvent::SyncStarted { start_height: 100 };
        assert_eq!(event, SyncEvent::SyncStarted { start_height: 100 });
    }

    #[test]
    fn test_sync_event_headers_progress() {
        let event = SyncEvent::HeadersProgress {
            current_height: 500,
            target_height: 1000,
        };
        assert_eq!(
            event,
            SyncEvent::HeadersProgress {
                current_height: 500,
                target_height: 1000,
            }
        );
    }

    #[test]
    fn test_sync_event_filtered_block_applied() {
        let event = SyncEvent::FilteredBlockApplied {
            height: 42,
            matched_tx_count: 3,
        };
        assert_eq!(
            event,
            SyncEvent::FilteredBlockApplied {
                height: 42,
                matched_tx_count: 3,
            }
        );
    }

    #[test]
    fn test_sync_event_capability_changed() {
        let event = SyncEvent::CapabilityChanged(SyncCapability::FullBip37);
        assert_eq!(
            event,
            SyncEvent::CapabilityChanged(SyncCapability::FullBip37)
        );
    }

    #[test]
    fn test_sync_event_sync_completed() {
        let event = SyncEvent::SyncCompleted { height: 800_000 };
        assert_eq!(event, SyncEvent::SyncCompleted { height: 800_000 });
    }

    #[test]
    fn test_sync_event_sync_failed() {
        let event = SyncEvent::SyncFailed(SyncError::NoBip37Peers);
        assert_eq!(event, SyncEvent::SyncFailed(SyncError::NoBip37Peers));
    }

    #[test]
    fn test_sync_error_variants() {
        assert_eq!(SyncError::NoBip37Peers, SyncError::NoBip37Peers);
        assert_eq!(
            SyncError::MaxConnectionFailures { count: 5 },
            SyncError::MaxConnectionFailures { count: 5 }
        );
        assert_ne!(
            SyncError::MaxConnectionFailures { count: 5 },
            SyncError::MaxConnectionFailures { count: 10 }
        );
        assert_eq!(SyncError::NetworkUnreachable, SyncError::NetworkUnreachable);
        assert_eq!(
            SyncError::PersistenceError("disk full".to_string()),
            SyncError::PersistenceError("disk full".to_string())
        );
        assert_ne!(
            SyncError::PersistenceError("a".to_string()),
            SyncError::PersistenceError("b".to_string())
        );
    }

    #[test]
    fn test_sync_event_clone() {
        let addr: SocketAddr = "10.0.0.1:8333".parse().unwrap();
        let event = SyncEvent::PeerConnected(addr);
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_sync_event_inequality() {
        let addr1: SocketAddr = "127.0.0.1:8333".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.2:8333".parse().unwrap();
        assert_ne!(
            SyncEvent::PeerConnected(addr1),
            SyncEvent::PeerConnected(addr2)
        );
        assert_ne!(
            SyncEvent::PeerConnected(addr1),
            SyncEvent::PeerDisconnected(addr1)
        );
    }
}
