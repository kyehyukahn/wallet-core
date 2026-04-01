// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::events::{SyncCapability, SyncError, SyncEvent};
use crate::peer::PeerInfo;
use std::net::SocketAddr;

const PEER_MAX_CONNECTIONS: usize = 3;
const MAX_CONNECT_FAILURES: u32 = 20;
const MAX_MISBEHAVIN_COUNT: u32 = 10;

/// A peer that has been connected and completed handshake.
#[derive(Debug, Clone)]
struct ConnectedPeer {
    info: PeerInfo,
    remote_start_height: u32,
    supports_bloom: bool,
    /// Will be used by SyncManager to track per-peer sync completion.
    #[allow(dead_code)]
    is_synced: bool,
}

/// Manages a pool of known and connected peers, and selects
/// the best download peer for block/header synchronisation.
pub struct PeerPool {
    known_peers: Vec<PeerInfo>,
    connected: Vec<ConnectedPeer>,
    download_peer_idx: Option<usize>,
    connect_failure_count: u32,
    misbehavin_count: u32,
    max_connections: usize,
    capability: SyncCapability,
}

impl Default for PeerPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerPool {
    /// Creates a new empty peer pool with no capability.
    pub fn new() -> Self {
        PeerPool {
            known_peers: Vec::new(),
            connected: Vec::new(),
            download_peer_idx: None,
            connect_failure_count: 0,
            misbehavin_count: 0,
            max_connections: PEER_MAX_CONNECTIONS,
            capability: SyncCapability::Unsupported,
        }
    }

    /// Loads peers from a cache, sorting by timestamp descending (most recent first).
    pub fn load_peers(&mut self, mut peers: Vec<PeerInfo>) {
        peers.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        self.known_peers = peers;
    }

    /// Merges newly discovered peers into the known set, deduplicating by address.
    pub fn add_discovered_peers(&mut self, peers: Vec<PeerInfo>) {
        for peer in peers {
            if !self.known_peers.iter().any(|p| p.address == peer.address) {
                self.known_peers.push(peer);
            }
        }
    }

    /// Returns up to `max_connections - connected_count` peers that are not
    /// already connected, suitable for initiating new connections.
    pub fn peers_to_connect(&self) -> Vec<PeerInfo> {
        let slots = self.max_connections.saturating_sub(self.connected.len());
        if slots == 0 {
            return Vec::new();
        }
        self.known_peers
            .iter()
            .filter(|p| !self.connected.iter().any(|c| c.info.address == p.address))
            .take(slots)
            .cloned()
            .collect()
    }

    /// Called when a peer completes its handshake. Adds the peer to the connected
    /// set, resets the failure counter, updates capability, and selects the
    /// download peer. Returns any events that should be emitted.
    pub fn on_peer_connected(
        &mut self,
        info: PeerInfo,
        remote_start_height: u32,
        supports_bloom: bool,
    ) -> Vec<SyncEvent> {
        let mut events = Vec::new();

        let connected_peer = ConnectedPeer {
            info: info.clone(),
            remote_start_height,
            supports_bloom,
            is_synced: false,
        };
        self.connected.push(connected_peer);
        self.connect_failure_count = 0;

        events.push(SyncEvent::PeerConnected(info.address));

        self.update_capability(&mut events);
        self.select_download_peer();

        events
    }

    /// Called when a peer disconnects. Removes the peer, reselects the download
    /// peer, and returns events.
    pub fn on_peer_disconnected(&mut self, addr: SocketAddr) -> Vec<SyncEvent> {
        let mut events = Vec::new();

        self.connected.retain(|c| c.info.address != addr);
        events.push(SyncEvent::PeerDisconnected(addr));

        self.update_capability(&mut events);
        self.select_download_peer();

        events
    }

    /// Called when a connection attempt fails. Increments the failure counter
    /// and emits `SyncFailed` when the threshold is reached.
    pub fn on_connect_failure(&mut self) -> Vec<SyncEvent> {
        self.connect_failure_count += 1;
        if self.connect_failure_count >= MAX_CONNECT_FAILURES {
            vec![SyncEvent::SyncFailed(SyncError::MaxConnectionFailures {
                count: self.connect_failure_count,
            })]
        } else {
            Vec::new()
        }
    }

    /// Called when a peer misbehaves. Removes the peer from connected and known
    /// sets, increments the misbehavin counter. Returns `true` if the counter
    /// has reached the threshold (meaning all peers should be cleared and a
    /// DNS re-query is needed).
    pub fn on_peer_misbehaving(&mut self, addr: SocketAddr) -> bool {
        self.connected.retain(|c| c.info.address != addr);
        self.known_peers.retain(|p| p.address != addr);
        self.misbehavin_count += 1;

        if self.misbehavin_count >= MAX_MISBEHAVIN_COUNT {
            self.known_peers.clear();
            self.connected.clear();
            self.download_peer_idx = None;
            self.misbehavin_count = 0;
            true
        } else {
            self.select_download_peer();
            false
        }
    }

    /// Returns the current download peer, if any.
    pub fn download_peer(&self) -> Option<&PeerInfo> {
        self.download_peer_idx
            .and_then(|idx| self.connected.get(idx))
            .map(|cp| &cp.info)
    }

    /// Returns the current sync capability.
    pub fn capability(&self) -> SyncCapability {
        self.capability
    }

    /// Returns the number of currently connected peers.
    pub fn connected_count(&self) -> usize {
        self.connected.len()
    }

    /// Returns the maximum start height reported by any connected peer.
    pub fn estimated_height(&self) -> u32 {
        self.connected
            .iter()
            .map(|c| c.remote_start_height)
            .max()
            .unwrap_or(0)
    }

    /// Selects the best download peer. Prefers a bloom-supporting peer with the
    /// highest start height; falls back to any peer for header-only sync.
    fn select_download_peer(&mut self) {
        if self.connected.is_empty() {
            self.download_peer_idx = None;
            return;
        }

        // Prefer bloom-supporting peer with highest start height.
        let bloom_best = self
            .connected
            .iter()
            .enumerate()
            .filter(|(_, c)| c.supports_bloom)
            .max_by_key(|(_, c)| c.remote_start_height);

        if let Some((idx, _)) = bloom_best {
            self.download_peer_idx = Some(idx);
            return;
        }

        // Fall back to any peer with highest start height (header-only).
        let best = self
            .connected
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| c.remote_start_height);

        self.download_peer_idx = best.map(|(idx, _)| idx);
    }

    /// Updates the sync capability based on connected peers and appends a
    /// `CapabilityChanged` event if the capability changed.
    fn update_capability(&mut self, events: &mut Vec<SyncEvent>) {
        let new_cap = if self.connected.iter().any(|c| c.supports_bloom) {
            SyncCapability::FullBip37
        } else if !self.connected.is_empty() {
            SyncCapability::HeaderOnly
        } else {
            SyncCapability::Unsupported
        };

        if new_cap != self.capability {
            self.capability = new_cap;
            events.push(SyncEvent::CapabilityChanged(new_cap));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::SERVICES_NODE_BLOOM;

    fn make_peer(addr: &str, services: u64, timestamp: u64) -> PeerInfo {
        PeerInfo {
            address: addr.parse().unwrap(),
            services,
            timestamp,
        }
    }

    #[test]
    fn test_peer_pool_initial() {
        let pool = PeerPool::new();
        assert_eq!(pool.capability(), SyncCapability::Unsupported);
        assert!(pool.download_peer().is_none());
        assert_eq!(pool.connected_count(), 0);
        assert_eq!(pool.estimated_height(), 0);
    }

    #[test]
    fn test_load_and_select_peers() {
        let mut pool = PeerPool::new();
        let peers = vec![
            make_peer("10.0.0.1:8333", SERVICES_NODE_BLOOM, 1000),
            make_peer("10.0.0.2:8333", SERVICES_NODE_BLOOM, 2000),
            make_peer("10.0.0.3:8333", SERVICES_NODE_BLOOM, 3000),
            make_peer("10.0.0.4:8333", SERVICES_NODE_BLOOM, 4000),
        ];
        pool.load_peers(peers);

        let to_connect = pool.peers_to_connect();
        // Max connections is 3, none connected yet, so we get 3 peers.
        assert_eq!(to_connect.len(), 3);
        // Should be sorted by timestamp desc, so highest first.
        assert_eq!(
            to_connect[0].address,
            "10.0.0.4:8333".parse::<SocketAddr>().unwrap()
        );
    }

    #[test]
    fn test_download_peer_prefers_bloom_with_highest_height() {
        let mut pool = PeerPool::new();

        // Connect two bloom peers and one non-bloom peer.
        let p1 = make_peer("10.0.0.1:8333", SERVICES_NODE_BLOOM, 1000);
        let p2 = make_peer("10.0.0.2:8333", SERVICES_NODE_BLOOM, 2000);
        let p3 = make_peer("10.0.0.3:8333", 0x01, 3000); // NODE_NETWORK only

        pool.on_peer_connected(p1, 800_000, true);
        pool.on_peer_connected(p2, 810_000, true);
        // Non-bloom peer has highest height but should not be preferred.
        pool.on_peer_connected(p3, 820_000, false);

        let dp = pool.download_peer().unwrap();
        // Should select bloom peer with highest height (810_000).
        assert_eq!(dp.address, "10.0.0.2:8333".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_capability_transitions() {
        let mut pool = PeerPool::new();
        assert_eq!(pool.capability(), SyncCapability::Unsupported);

        // Connect a non-bloom peer -> HeaderOnly.
        let p1 = make_peer("10.0.0.1:8333", 0x01, 1000);
        let events = pool.on_peer_connected(p1, 800_000, false);
        assert_eq!(pool.capability(), SyncCapability::HeaderOnly);
        assert!(events.contains(&SyncEvent::CapabilityChanged(SyncCapability::HeaderOnly)));

        // Connect a bloom peer -> FullBip37.
        let p2 = make_peer("10.0.0.2:8333", SERVICES_NODE_BLOOM, 2000);
        let events = pool.on_peer_connected(p2, 810_000, true);
        assert_eq!(pool.capability(), SyncCapability::FullBip37);
        assert!(events.contains(&SyncEvent::CapabilityChanged(SyncCapability::FullBip37)));

        // Disconnect the bloom peer -> back to HeaderOnly.
        let addr: SocketAddr = "10.0.0.2:8333".parse().unwrap();
        let events = pool.on_peer_disconnected(addr);
        assert_eq!(pool.capability(), SyncCapability::HeaderOnly);
        assert!(events.contains(&SyncEvent::CapabilityChanged(SyncCapability::HeaderOnly)));
    }

    #[test]
    fn test_misbehaving_clears_peers() {
        let mut pool = PeerPool::new();

        // Add some known peers and connect one.
        for i in 1..=12 {
            let peer = make_peer(&format!("10.0.0.{}:8333", i), SERVICES_NODE_BLOOM, 1000);
            pool.known_peers.push(peer);
        }
        let p = make_peer("10.0.0.1:8333", SERVICES_NODE_BLOOM, 1000);
        pool.on_peer_connected(p, 800_000, true);

        // 9 misbehaving events should not clear.
        for i in 1..MAX_MISBEHAVIN_COUNT {
            let addr: SocketAddr = format!("10.0.0.{}:8333", i).parse().unwrap();
            let cleared = pool.on_peer_misbehaving(addr);
            assert!(!cleared, "should not clear at count {}", i);
        }

        // 10th misbehaving event triggers clear.
        let addr: SocketAddr = format!("10.0.0.{}:8333", MAX_MISBEHAVIN_COUNT)
            .parse()
            .unwrap();
        let cleared = pool.on_peer_misbehaving(addr);
        assert!(cleared);
        assert!(pool.known_peers.is_empty());
        assert!(pool.connected.is_empty());
        assert!(pool.download_peer().is_none());
    }

    #[test]
    fn test_connect_failure_threshold() {
        let mut pool = PeerPool::new();

        // 19 failures should produce no events.
        for _ in 0..(MAX_CONNECT_FAILURES - 1) {
            let events = pool.on_connect_failure();
            assert!(events.is_empty());
        }

        // 20th failure should emit SyncFailed.
        let events = pool.on_connect_failure();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SyncEvent::SyncFailed(SyncError::MaxConnectionFailures {
                count: MAX_CONNECT_FAILURES,
            })
        );
    }

    #[test]
    fn test_estimated_height() {
        let mut pool = PeerPool::new();
        assert_eq!(pool.estimated_height(), 0);

        let p1 = make_peer("10.0.0.1:8333", SERVICES_NODE_BLOOM, 1000);
        let p2 = make_peer("10.0.0.2:8333", SERVICES_NODE_BLOOM, 2000);
        let p3 = make_peer("10.0.0.3:8333", SERVICES_NODE_BLOOM, 3000);

        pool.on_peer_connected(p1, 800_000, true);
        pool.on_peer_connected(p2, 810_000, true);
        pool.on_peer_connected(p3, 805_000, true);

        assert_eq!(pool.estimated_height(), 810_000);
    }
}
