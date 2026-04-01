// SPDX-License-Identifier: Apache-2.0

use crate::chain::{BlockHeader, ChainId, Checkpoint, HeaderVerificationError, SpvChain};
use tw_hash::H256;

/// Maximum allowed clock drift for block timestamps (2 hours in seconds).
const MAX_FUTURE_BLOCK_TIME: u32 = 2 * 60 * 60;

static MAINNET_CHECKPOINTS: &[Checkpoint] = &[Checkpoint {
    height: 0,
    hash: H256::from_array([0x12; 32]),
    timestamp: 1317972665,
    target: 0x1e0ffff0,
}];

static TESTNET_CHECKPOINTS: &[Checkpoint] = &[Checkpoint {
    height: 0,
    hash: H256::from_array([0x13; 32]),
    timestamp: 1486949366,
    target: 0x1e0ffff0,
}];

static MAINNET_DNS_SEEDS: &[&str] = &[
    "seed-a.litecoin.loshan.co.uk",
    "dnsseed.thrasher.io",
    "dnsseed.litecointools.com",
    "dnsseed.litecoinpool.org",
];

static TESTNET_DNS_SEEDS: &[&str] = &[
    "testnet-seed.litecointools.com",
    "seed-b.litecoin.loshan.co.uk",
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

pub struct LitecoinMainnet;

impl SpvChain for LitecoinMainnet {
    fn chain_id(&self) -> ChainId {
        ChainId::LitecoinMainnet
    }

    fn network_magic(&self) -> u32 {
        0xdbb6c0fb
    }

    fn default_port(&self) -> u16 {
        9333
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

    /// Scrypt PoW verified at higher layer; timestamp check only.
    fn verify_header_chain(
        &self,
        _prev: &BlockHeader,
        current: &BlockHeader,
        _height: u32,
    ) -> Result<(), HeaderVerificationError> {
        verify_header_timestamp(current)
    }
}

pub struct LitecoinTestnet;

impl SpvChain for LitecoinTestnet {
    fn chain_id(&self) -> ChainId {
        ChainId::LitecoinTestnet
    }

    fn network_magic(&self) -> u32 {
        0xf1c8d2fd
    }

    fn default_port(&self) -> u16 {
        19335
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
    fn test_litecoin_mainnet_config() {
        let chain = LitecoinMainnet;
        assert_eq!(chain.chain_id(), ChainId::LitecoinMainnet);
        assert_eq!(chain.network_magic(), 0xdbb6c0fb);
        assert_eq!(chain.default_port(), 9333);
        assert_eq!(chain.dns_seeds().len(), 4);
        assert_eq!(chain.protocol_version(), 70015);
        assert!(chain.supports_bloom_filtering());
        assert!(!chain.checkpoints().is_empty());
    }

    #[test]
    fn test_litecoin_testnet_config() {
        let chain = LitecoinTestnet;
        assert_eq!(chain.chain_id(), ChainId::LitecoinTestnet);
        assert_eq!(chain.network_magic(), 0xf1c8d2fd);
        assert_eq!(chain.default_port(), 19335);
        assert_eq!(chain.protocol_version(), 70015);
        assert!(chain.supports_bloom_filtering());
        assert!(!chain.checkpoints().is_empty());
    }

    #[test]
    fn test_litecoin_genesis_checkpoint() {
        let chain = LitecoinMainnet;
        let genesis = &chain.checkpoints()[0];
        assert_eq!(genesis.height, 0);
        assert_eq!(genesis.timestamp, 1317972665);
        assert_eq!(genesis.target, 0x1e0ffff0);
    }

    #[test]
    fn test_litecoin_different_from_bitcoin() {
        let btc = BitcoinMainnet;
        let ltc = LitecoinMainnet;
        assert_ne!(btc.chain_id(), ltc.chain_id());
        assert_ne!(btc.network_magic(), ltc.network_magic());
        assert_ne!(btc.default_port(), ltc.default_port());
    }
}
