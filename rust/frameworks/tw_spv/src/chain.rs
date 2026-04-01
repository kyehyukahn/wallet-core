// SPDX-License-Identifier: Apache-2.0

use tw_hash::H256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainId {
    BitcoinMainnet,
    BitcoinTestnet,
}

#[derive(Debug, Clone, Copy)]
pub struct Checkpoint {
    pub height: u32,
    pub hash: H256,
    pub timestamp: u32,
    pub target: u32,
}

#[derive(Debug, Clone, Default)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block: H256,
    pub merkle_root: H256,
    pub timestamp: u32,
    pub target: u32,
    pub nonce: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderVerificationError {
    TimestampTooFarInFuture,
    InvalidDifficulty,
    InvalidProofOfWork,
    NonContiguousChain,
}

/// Chain-level configuration and verification rules for SPV sync.
/// `supports_bloom_filtering()` is a chain-level policy.
/// Per-peer NODE_BLOOM service bit checking is handled in PeerManager.
pub trait SpvChain {
    fn chain_id(&self) -> ChainId;
    fn network_magic(&self) -> u32;
    fn default_port(&self) -> u16;
    fn dns_seeds(&self) -> &'static [&'static str];
    fn checkpoints(&self) -> &[Checkpoint];
    fn min_protocol_version(&self) -> u32;
    fn protocol_version(&self) -> u32;
    fn supports_bloom_filtering(&self) -> bool;
    fn verify_header_chain(
        &self,
        prev: &BlockHeader,
        current: &BlockHeader,
        height: u32,
    ) -> Result<(), HeaderVerificationError>;
}
