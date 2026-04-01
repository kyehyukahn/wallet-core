// SPDX-License-Identifier: Apache-2.0

use crate::chain::{BlockHeader, ChainId, Checkpoint, HeaderVerificationError, SpvChain};
use tw_hash::H256;

/// Bitcoin Cash fork ID used for replay-protected transaction signing.
pub const BCASH_FORK_ID: u8 = 0x40;

/// Maximum allowed clock drift for block timestamps (2 hours in seconds).
const MAX_FUTURE_BLOCK_TIME: u32 = 2 * 60 * 60;

// Bitcoin Cash forked from Bitcoin at block 478559.
// The genesis block is the same as Bitcoin's genesis block.
static MAINNET_CHECKPOINTS: &[Checkpoint] = &[Checkpoint {
    height: 0,
    hash: H256::from_array([
        0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7,
        0x4f, 0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]),
    timestamp: 1231006505,
    target: 0x1d00ffff,
}];

static TESTNET_CHECKPOINTS: &[Checkpoint] = &[Checkpoint {
    height: 0,
    hash: H256::from_array([
        0x43, 0x49, 0x7f, 0xd7, 0xf8, 0x26, 0x95, 0x71, 0x08, 0xf4, 0xa3, 0x0f, 0xd9, 0xce, 0xc3,
        0xae, 0xba, 0x79, 0x97, 0x20, 0x84, 0xe9, 0x0e, 0xad, 0x01, 0xea, 0x33, 0x09, 0x00, 0x00,
        0x00, 0x00,
    ]),
    timestamp: 1296688602,
    target: 0x1d00ffff,
}];

static MAINNET_DNS_SEEDS: &[&str] = &[
    "seed.bitcoinabc.org",
    "seed-abc.bitcoinforks.org",
    "btccash-seeder.bitcoinunlimited.info",
    "seed.bitprim.org",
    "seed.deadalnix.me",
];

static TESTNET_DNS_SEEDS: &[&str] = &[
    "testnet-seed.bitcoinabc.org",
    "testnet-seed-abc.bitcoinforks.org",
];

/// Returns the current wall-clock time as a Unix timestamp.
fn current_timestamp() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs() as u32
}

fn verify_header_timestamp(current: &BlockHeader) -> Result<(), HeaderVerificationError> {
    let now = current_timestamp();
    if current.timestamp > now + MAX_FUTURE_BLOCK_TIME {
        return Err(HeaderVerificationError::TimestampTooFarInFuture);
    }
    Ok(())
}

pub struct BitcoinCashMainnet;

impl SpvChain for BitcoinCashMainnet {
    fn chain_id(&self) -> ChainId {
        ChainId::BitcoinCashMainnet
    }

    fn network_magic(&self) -> u32 {
        0xe8f3e1e3
    }

    fn default_port(&self) -> u16 {
        8333
    }

    fn dns_seeds(&self) -> &'static [&'static str] {
        MAINNET_DNS_SEEDS
    }

    fn checkpoints(&self) -> &[Checkpoint] {
        MAINNET_CHECKPOINTS
    }

    fn min_protocol_version(&self) -> u32 {
        70002
    }

    fn protocol_version(&self) -> u32 {
        70015
    }

    fn supports_bloom_filtering(&self) -> bool {
        true
    }

    /// BCash DAA is complex; timestamp check only, full PoW deferred to sync layer.
    fn verify_header_chain(
        &self,
        _prev: &BlockHeader,
        current: &BlockHeader,
        _height: u32,
    ) -> Result<(), HeaderVerificationError> {
        verify_header_timestamp(current)
    }
}

pub struct BitcoinCashTestnet;

impl SpvChain for BitcoinCashTestnet {
    fn chain_id(&self) -> ChainId {
        ChainId::BitcoinCashTestnet
    }

    fn network_magic(&self) -> u32 {
        0xf4f3e5f4
    }

    fn default_port(&self) -> u16 {
        18333
    }

    fn dns_seeds(&self) -> &'static [&'static str] {
        TESTNET_DNS_SEEDS
    }

    fn checkpoints(&self) -> &[Checkpoint] {
        TESTNET_CHECKPOINTS
    }

    fn min_protocol_version(&self) -> u32 {
        70002
    }

    fn protocol_version(&self) -> u32 {
        70015
    }

    fn supports_bloom_filtering(&self) -> bool {
        true
    }

    fn verify_header_chain(
        &self,
        _prev: &BlockHeader,
        current: &BlockHeader,
        _height: u32,
    ) -> Result<(), HeaderVerificationError> {
        verify_header_timestamp(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin_chain::BitcoinMainnet;

    #[test]
    fn test_bcash_mainnet_config() {
        let chain = BitcoinCashMainnet;
        assert_eq!(chain.chain_id(), ChainId::BitcoinCashMainnet);
        assert_eq!(chain.network_magic(), 0xe8f3e1e3);
        assert_eq!(chain.default_port(), 8333);
        assert_eq!(chain.dns_seeds().len(), 5);
        assert!(chain.dns_seeds().contains(&"seed.bitcoinabc.org"));
        assert_eq!(chain.protocol_version(), 70015);
        assert!(chain.supports_bloom_filtering());
        assert!(!chain.checkpoints().is_empty());
    }

    #[test]
    fn test_bcash_testnet_config() {
        let chain = BitcoinCashTestnet;
        assert_eq!(chain.chain_id(), ChainId::BitcoinCashTestnet);
        assert_eq!(chain.network_magic(), 0xf4f3e5f4);
        assert_eq!(chain.default_port(), 18333);
        assert_eq!(chain.dns_seeds().len(), 2);
        assert_eq!(chain.protocol_version(), 70015);
        assert!(chain.supports_bloom_filtering());
        assert!(!chain.checkpoints().is_empty());
    }

    #[test]
    fn test_bcash_different_from_bitcoin() {
        let btc = BitcoinMainnet;
        let bch = BitcoinCashMainnet;
        assert_ne!(btc.chain_id(), bch.chain_id());
        assert_ne!(btc.network_magic(), bch.network_magic());
    }

    #[test]
    fn test_bcash_fork_id_value() {
        assert_eq!(BCASH_FORK_ID, 0x40);
    }

    #[test]
    fn test_bcash_timestamp_validation() {
        let chain = BitcoinCashMainnet;
        let prev = BlockHeader::default();

        let valid = BlockHeader {
            timestamp: current_timestamp(),
            ..Default::default()
        };
        assert!(chain.verify_header_chain(&prev, &valid, 1).is_ok());

        let future = BlockHeader {
            timestamp: current_timestamp() + 3 * 60 * 60,
            ..Default::default()
        };
        assert_eq!(
            chain.verify_header_chain(&prev, &future, 1),
            Err(HeaderVerificationError::TimestampTooFarInFuture)
        );
    }
}
