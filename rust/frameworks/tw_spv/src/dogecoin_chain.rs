// SPDX-License-Identifier: Apache-2.0

use crate::chain::{BlockHeader, ChainId, Checkpoint, HeaderVerificationError, SpvChain};
use tw_hash::H256;

/// Maximum allowed clock drift for block timestamps (2 hours in seconds).
const MAX_FUTURE_BLOCK_TIME: u32 = 2 * 60 * 60;

static MAINNET_CHECKPOINTS: &[Checkpoint] = &[Checkpoint {
    height: 0,
    hash: H256::from_array([0x1b; 32]),
    timestamp: 1386325540,
    target: 0x1e0ffff0,
}];

static TESTNET_CHECKPOINTS: &[Checkpoint] = &[Checkpoint {
    height: 0,
    hash: H256::from_array([0x1c; 32]),
    timestamp: 1391503289,
    target: 0x1e0ffff0,
}];

static MAINNET_DNS_SEEDS: &[&str] = &[
    "seed.multidoge.org",
    "seed2.multidoge.org",
    "seed.doger.dogecoin.com",
    "seed.dogecoin.com",
];

static TESTNET_DNS_SEEDS: &[&str] = &["testseed.jrn.me.uk", "seed.testnet.dogecoin.com"];

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

pub struct DogecoinMainnet;

impl SpvChain for DogecoinMainnet {
    fn chain_id(&self) -> ChainId {
        ChainId::DogecoinMainnet
    }

    fn network_magic(&self) -> u32 {
        0xc0c0c0c0
    }

    fn default_port(&self) -> u16 {
        22556
    }

    fn dns_seeds(&self) -> &'static [&'static str] {
        MAINNET_DNS_SEEDS
    }

    fn checkpoints(&self) -> &[Checkpoint] {
        MAINNET_CHECKPOINTS
    }

    fn min_protocol_version(&self) -> u32 {
        70003
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

pub struct DogecoinTestnet;

impl SpvChain for DogecoinTestnet {
    fn chain_id(&self) -> ChainId {
        ChainId::DogecoinTestnet
    }

    fn network_magic(&self) -> u32 {
        0xfcc1b7dc
    }

    fn default_port(&self) -> u16 {
        44556
    }

    fn dns_seeds(&self) -> &'static [&'static str] {
        TESTNET_DNS_SEEDS
    }

    fn checkpoints(&self) -> &[Checkpoint] {
        TESTNET_CHECKPOINTS
    }

    fn min_protocol_version(&self) -> u32 {
        70003
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
    use crate::bcash_chain::BitcoinCashMainnet;
    use crate::bitcoin_chain::BitcoinMainnet;
    use crate::litecoin_chain::LitecoinMainnet;

    #[test]
    fn test_dogecoin_mainnet_config() {
        let chain = DogecoinMainnet;
        assert_eq!(chain.chain_id(), ChainId::DogecoinMainnet);
        assert_eq!(chain.network_magic(), 0xc0c0c0c0);
        assert_eq!(chain.default_port(), 22556);
        assert_eq!(chain.dns_seeds().len(), 4);
        assert_eq!(chain.protocol_version(), 70015);
        assert_eq!(chain.min_protocol_version(), 70003);
        assert!(chain.supports_bloom_filtering());
        assert!(!chain.checkpoints().is_empty());
    }

    #[test]
    fn test_dogecoin_testnet_config() {
        let chain = DogecoinTestnet;
        assert_eq!(chain.chain_id(), ChainId::DogecoinTestnet);
        assert_eq!(chain.network_magic(), 0xfcc1b7dc);
        assert_eq!(chain.default_port(), 44556);
        assert_eq!(chain.protocol_version(), 70015);
        assert_eq!(chain.min_protocol_version(), 70003);
        assert!(chain.supports_bloom_filtering());
        assert!(!chain.checkpoints().is_empty());
    }

    #[test]
    fn test_dogecoin_genesis_checkpoint() {
        let chain = DogecoinMainnet;
        let genesis = &chain.checkpoints()[0];
        assert_eq!(genesis.height, 0);
        assert_eq!(genesis.timestamp, 1386325540);
        assert_eq!(genesis.target, 0x1e0ffff0);
    }

    #[test]
    fn test_dogecoin_unique_magic() {
        let btc = BitcoinMainnet;
        let ltc = LitecoinMainnet;
        let bch = BitcoinCashMainnet;
        let doge = DogecoinMainnet;

        assert_ne!(doge.network_magic(), btc.network_magic());
        assert_ne!(doge.network_magic(), ltc.network_magic());
        assert_ne!(doge.network_magic(), bch.network_magic());
    }
}
