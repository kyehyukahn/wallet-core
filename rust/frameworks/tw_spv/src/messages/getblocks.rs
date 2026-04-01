// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::SpvResult;
use tw_hash::H256;
use tw_utxo::encode::compact_integer::CompactInteger;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

/// Block locator message used by `getblocks` and `getheaders`.
///
/// Wire format: version(4) + count(varint) + hashes(count*32) + hash_stop(32)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockLocatorMessage {
    pub version: u32,
    pub locator_hashes: Vec<H256>,
    pub hash_stop: H256,
}

impl BlockLocatorMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        stream.append(&self.version);
        CompactInteger::from(self.locator_hashes.len()).encode(&mut stream);
        for hash in &self.locator_hashes {
            stream.append(hash);
        }
        stream.append(&self.hash_stop);
        stream.out()
    }

    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let version = reader.read_u32_le()?;
        let count = reader.read_compact_int()? as usize;
        let mut locator_hashes = Vec::with_capacity(count);
        for _ in 0..count {
            locator_hashes.push(reader.read_h256()?);
        }
        let hash_stop = reader.read_h256()?;
        Ok(BlockLocatorMessage {
            version,
            locator_hashes,
            hash_stop,
        })
    }
}

/// A `getblocks` message has the same format as a block locator message.
pub type GetBlocksMessage = BlockLocatorMessage;

/// A `getheaders` message has the same format as a block locator message.
pub type GetHeadersMessage = BlockLocatorMessage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_locator_roundtrip() {
        let msg = BlockLocatorMessage {
            version: 70015,
            locator_hashes: vec![H256::from([0x11; 32]), H256::from([0x22; 32])],
            hash_stop: H256::from([0x00; 32]),
        };
        let serialized = msg.serialize();
        let parsed = BlockLocatorMessage::parse(&serialized).unwrap();
        assert_eq!(parsed, msg);
    }
}
