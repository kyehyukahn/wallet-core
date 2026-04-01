// SPDX-License-Identifier: Apache-2.0

use crate::chain::{BlockHeader, Checkpoint, SpvChain};
use crate::events::{SyncCapability, SyncEvent};
use std::collections::HashMap;
use tw_hash::H256;

/// One week in seconds, used to determine when header sync is close enough
/// to the wallet's earliest key time to transition to filtered block download.
const ONE_WEEK_SECS: u32 = 7 * 24 * 60 * 60;

/// Maximum headers per `headers` message (Bitcoin protocol limit).
const MAX_HEADERS_PER_MESSAGE: usize = 2000;

// ---------------------------------------------------------------------------
// FilterStrategy trait (reserved for future BIP157/158 support)
// ---------------------------------------------------------------------------

/// Strategy interface for requesting and processing filtered block data.
/// Currently reserved for future BIP157/158 compact-filter support.
pub trait FilterStrategy: Send + Sync {
    fn filter_type(&self) -> SyncCapability;
    fn build_filter_message(&self, addresses: &[&str]) -> Vec<u8>;
    fn process_filtered_response(&self, data: &[u8]) -> Vec<H256>;
}

// ---------------------------------------------------------------------------
// SyncPhase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPhase {
    Idle,
    HeaderSync,
    FilteredBlockSync,
    Monitoring,
}

// ---------------------------------------------------------------------------
// StoredHeader (internal)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StoredHeader {
    header: BlockHeader,
    height: u32,
    hash: H256,
}

// ---------------------------------------------------------------------------
// SyncState
// ---------------------------------------------------------------------------

pub struct SyncState {
    phase: SyncPhase,
    earliest_key_time: u32,
    sync_start_height: u32,
    headers: HashMap<H256, StoredHeader>,
    height_to_hash: HashMap<u32, H256>,
    last_header: Option<StoredHeader>,
    checkpoints: HashMap<H256, Checkpoint>,
    last_height: u32,
}

impl SyncState {
    /// Create a new `SyncState` seeded with the given chain's checkpoints.
    pub fn new<C: SpvChain>(chain: &C, earliest_key_time: u32) -> Self {
        let checkpoint_list = chain.checkpoints();
        let mut checkpoints = HashMap::new();
        for cp in checkpoint_list {
            checkpoints.insert(cp.hash, *cp);
        }
        SyncState {
            phase: SyncPhase::Idle,
            earliest_key_time,
            sync_start_height: 0,
            headers: HashMap::new(),
            height_to_hash: HashMap::new(),
            last_header: None,
            checkpoints,
            last_height: 0,
        }
    }

    pub fn phase(&self) -> SyncPhase {
        self.phase
    }

    pub fn last_height(&self) -> u32 {
        self.last_height
    }

    /// Begin synchronisation: transitions to `HeaderSync` and emits `SyncStarted`.
    pub fn start_sync(&mut self) -> Vec<SyncEvent> {
        self.phase = SyncPhase::HeaderSync;
        vec![SyncEvent::SyncStarted {
            start_height: self.last_height,
        }]
    }

    /// Build block-locator hashes using the breadwallet-core algorithm:
    ///
    /// Starting from the most recent block, emit up to 10 consecutive hashes,
    /// then double the step size for each subsequent hash, always finishing
    /// with the genesis block (height 0).
    pub fn block_locators(&self) -> Vec<H256> {
        let mut locators = Vec::new();
        let mut height = self.last_height;
        let mut step: u32 = 1;
        let mut count: u32 = 0;

        loop {
            if let Some(hash) = self.hash_at_height(height) {
                locators.push(hash);
            }
            count += 1;
            if count >= 10 {
                step = step.saturating_mul(2);
            }
            if height < step {
                break;
            }
            height -= step;
        }

        // Always include genesis.
        if let Some(genesis) = self.hash_at_height(0) {
            if locators.last() != Some(&genesis) {
                locators.push(genesis);
            }
        }

        locators
    }

    /// Ingest a batch of headers received from a peer.
    ///
    /// Returns a list of events and a boolean `need_more` indicating whether
    /// the caller should request the next batch.
    pub fn process_headers(
        &mut self,
        headers: Vec<BlockHeader>,
        target_height: u32,
    ) -> (Vec<SyncEvent>, bool) {
        let mut events = Vec::new();
        let header_count = headers.len();

        for header in headers {
            let hash = header.block_hash();
            let height = self.last_height + 1;
            let stored = StoredHeader {
                header,
                height,
                hash,
            };
            self.headers.insert(hash, stored.clone());
            self.height_to_hash.insert(height, hash);
            self.last_header = Some(stored);
            self.last_height = height;
        }

        events.push(SyncEvent::HeadersProgress {
            current_height: self.last_height,
            target_height,
        });

        // Determine whether we should continue requesting headers.
        let should_continue = self.should_continue_headers();
        let need_more = header_count >= MAX_HEADERS_PER_MESSAGE && should_continue;

        // If we have caught up close enough to the wallet creation time,
        // transition to filtered block download.
        if !should_continue {
            self.phase = SyncPhase::FilteredBlockSync;
        }

        (events, need_more)
    }

    /// Called when a filtered (merkle) block has been validated and applied.
    pub fn on_filtered_block_applied(
        &mut self,
        height: u32,
        matched_tx_count: usize,
        target_height: u32,
    ) -> Vec<SyncEvent> {
        let mut events = vec![SyncEvent::FilteredBlockApplied {
            height,
            matched_tx_count,
        }];

        if height >= target_height {
            self.phase = SyncPhase::Monitoring;
            events.push(SyncEvent::SyncCompleted { height });
        }

        events
    }

    /// Stop synchronisation and return to `Idle`.
    pub fn stop_sync(&mut self) {
        self.phase = SyncPhase::Idle;
    }

    /// Compute the overall sync progress as a value between 0.0 and 1.0.
    pub fn sync_progress(&self, target_height: u32) -> f64 {
        if target_height <= self.sync_start_height {
            return 1.0;
        }
        let range = (target_height - self.sync_start_height) as f64;
        let done = self.last_height.saturating_sub(self.sync_start_height) as f64;
        (done / range).clamp(0.0, 1.0)
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn hash_at_height(&self, height: u32) -> Option<H256> {
        if let Some(hash) = self.height_to_hash.get(&height) {
            return Some(*hash);
        }
        // Fall back to checkpoints.
        for cp in self.checkpoints.values() {
            if cp.height == height {
                return Some(cp.hash);
            }
        }
        None
    }

    /// Returns `true` when we should keep requesting headers.
    /// We stop when the latest header's timestamp is within one week of
    /// `earliest_key_time`.
    fn should_continue_headers(&self) -> bool {
        if let Some(ref last) = self.last_header {
            if last.header.timestamp + ONE_WEEK_SECS >= self.earliest_key_time {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin_chain::BitcoinMainnet;
    use crate::chain::{ChainId, HeaderVerificationError};

    /// Minimal chain implementation for testing.
    struct TestChain {
        checkpoints: Vec<Checkpoint>,
    }

    impl TestChain {
        fn new() -> Self {
            TestChain {
                checkpoints: vec![Checkpoint {
                    height: 0,
                    hash: H256::from([0x01; 32]),
                    timestamp: 1231006505,
                    target: 0x1d00ffff,
                }],
            }
        }
    }

    impl SpvChain for TestChain {
        fn chain_id(&self) -> ChainId {
            ChainId::BitcoinMainnet
        }
        fn network_magic(&self) -> u32 {
            0
        }
        fn default_port(&self) -> u16 {
            8333
        }
        fn dns_seeds(&self) -> &'static [&'static str] {
            &[]
        }
        fn checkpoints(&self) -> &[Checkpoint] {
            &self.checkpoints
        }
        fn min_protocol_version(&self) -> u32 {
            70002
        }
        fn protocol_version(&self) -> u32 {
            70013
        }
        fn supports_bloom_filtering(&self) -> bool {
            true
        }
        fn verify_header_chain(
            &self,
            _prev: &BlockHeader,
            _current: &BlockHeader,
            _height: u32,
        ) -> Result<(), HeaderVerificationError> {
            Ok(())
        }
    }

    /// Helper: create a header with a specific timestamp (and unique hash via nonce).
    fn make_header(timestamp: u32, nonce: u32) -> BlockHeader {
        BlockHeader {
            version: 0x20000000,
            prev_block: H256::default(),
            merkle_root: H256::default(),
            timestamp,
            target: 0x1d00ffff,
            nonce,
        }
    }

    // Test 1
    #[test]
    fn test_sync_state_initial() {
        let chain = TestChain::new();
        let state = SyncState::new(&chain, 0);
        assert_eq!(state.phase(), SyncPhase::Idle);
        assert_eq!(state.last_height(), 0);
    }

    // Test 2
    #[test]
    fn test_start_sync_emits_event() {
        let chain = TestChain::new();
        let mut state = SyncState::new(&chain, 0);
        let events = state.start_sync();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], SyncEvent::SyncStarted { start_height: 0 });
        assert_eq!(state.phase(), SyncPhase::HeaderSync);
    }

    // Test 3
    #[test]
    fn test_block_locators_algorithm() {
        let chain = TestChain::new();
        // Use a large earliest_key_time so headers don't trigger phase transition.
        let mut state = SyncState::new(&chain, u32::MAX);
        state.start_sync();

        // Simulate 20 headers.
        let headers: Vec<BlockHeader> = (0..20).map(|i| make_header(1_000_000 + i, i)).collect();
        let _ = state.process_headers(headers, 1000);

        let locators = state.block_locators();
        assert!(!locators.is_empty());

        // First locator should be at height 20 (most recent).
        assert_eq!(locators[0], *state.height_to_hash.get(&20).unwrap());

        // Last locator must be the genesis hash (height 0 checkpoint).
        let genesis_hash = H256::from([0x01; 32]);
        assert_eq!(*locators.last().unwrap(), genesis_hash);

        // Verify the breadwallet-core step pattern.
        // Heights should be: 20, 19, 18, 17, 16, 15, 14, 13, 12, 11,
        //                    then step doubles: 9, 5, genesis(0)
        let expected_heights: Vec<u32> = vec![20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 9, 5, 0];
        let actual_heights: Vec<u32> = locators
            .iter()
            .filter_map(|h| {
                state
                    .height_to_hash
                    .iter()
                    .find(|(_, v)| *v == h)
                    .map(|(k, _)| *k)
                    .or_else(|| {
                        state
                            .checkpoints
                            .values()
                            .find(|cp| cp.hash == *h)
                            .map(|cp| cp.height)
                    })
            })
            .collect();
        assert_eq!(actual_heights, expected_heights);
    }

    // Test 4
    #[test]
    fn test_process_headers_progress() {
        let chain = TestChain::new();
        let mut state = SyncState::new(&chain, u32::MAX);
        state.start_sync();

        let headers: Vec<BlockHeader> = (0..5).map(|i| make_header(1_000_000 + i, i)).collect();
        let (events, need_more) = state.process_headers(headers, 100);

        assert_eq!(state.last_height(), 5);
        assert!(!need_more); // Only 5 headers, below 2000 threshold.
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SyncEvent::HeadersProgress {
                current_height: 5,
                target_height: 100,
            }
        );
    }

    // Test 5
    #[test]
    fn test_sync_progress_calculation() {
        let chain = TestChain::new();
        let mut state = SyncState::new(&chain, u32::MAX);
        state.sync_start_height = 100;
        state.last_height = 150;

        let progress = state.sync_progress(200);
        assert!((progress - 0.5).abs() < f64::EPSILON);
    }

    // Test 6
    #[test]
    fn test_filtered_block_completion() {
        let chain = TestChain::new();
        let mut state = SyncState::new(&chain, 0);
        state.start_sync();
        state.phase = SyncPhase::FilteredBlockSync;

        let events = state.on_filtered_block_applied(500, 3, 500);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            SyncEvent::FilteredBlockApplied {
                height: 500,
                matched_tx_count: 3,
            }
        );
        assert_eq!(events[1], SyncEvent::SyncCompleted { height: 500 });
        assert_eq!(state.phase(), SyncPhase::Monitoring);
    }

    // Verify with the real BitcoinMainnet chain that checkpoints load correctly.
    #[test]
    fn test_new_with_bitcoin_mainnet() {
        let chain = BitcoinMainnet;
        let state = SyncState::new(&chain, 1231006505);
        assert_eq!(state.phase(), SyncPhase::Idle);
        assert!(!state.checkpoints.is_empty());
    }
}
