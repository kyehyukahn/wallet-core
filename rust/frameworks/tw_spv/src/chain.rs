// SPDX-License-Identifier: Apache-2.0

use crate::decode::{Decodable, Reader};
use crate::error::SpvResult;
use tw_hash::sha2::sha256_d;
use tw_hash::H256;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainId {
    BitcoinMainnet,
    BitcoinTestnet,
    BitcoinCashMainnet,
    BitcoinCashTestnet,
    LitecoinMainnet,
    LitecoinTestnet,
    DogecoinMainnet,
    DogecoinTestnet,
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

impl BlockHeader {
    pub const SIZE: usize = 80;

    pub fn block_hash(&self) -> H256 {
        let data = tw_utxo::encode::encode(self);
        let hash = sha256_d(&data);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        H256::from(arr)
    }
}

impl Encodable for BlockHeader {
    fn encode(&self, stream: &mut Stream) {
        stream
            .append(&self.version)
            .append(&self.prev_block)
            .append(&self.merkle_root)
            .append(&self.timestamp)
            .append(&self.target)
            .append(&self.nonce);
    }

    fn encoded_size(&self) -> usize {
        Self::SIZE
    }
}

impl Decodable for BlockHeader {
    fn decode(reader: &mut Reader<'_>) -> SpvResult<Self> {
        Ok(BlockHeader {
            version: reader.read_i32_le()?,
            prev_block: reader.read_h256()?,
            merkle_root: reader.read_h256()?,
            timestamp: reader.read_u32_le()?,
            target: reader.read_u32_le()?,
            nonce: reader.read_u32_le()?,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_roundtrip() {
        let header = BlockHeader {
            version: 0x20000000,
            prev_block: H256::from([0xAA; 32]),
            merkle_root: H256::from([0xBB; 32]),
            timestamp: 1231006505,
            target: 0x1d00ffff,
            nonce: 2083236893,
        };
        let encoded = tw_utxo::encode::encode(&header);
        assert_eq!(encoded.len(), 80);
        let mut reader = Reader::new(&encoded);
        let decoded = BlockHeader::decode(&mut reader).unwrap();
        assert_eq!(decoded.version, header.version);
        assert_eq!(decoded.timestamp, header.timestamp);
        assert_eq!(decoded.prev_block, header.prev_block);
        assert_eq!(decoded.merkle_root, header.merkle_root);
        assert_eq!(decoded.target, header.target);
        assert_eq!(decoded.nonce, header.nonce);
    }
}
