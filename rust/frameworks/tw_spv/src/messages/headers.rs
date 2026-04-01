// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::chain::BlockHeader;
use crate::decode::{Decodable, Reader};
use crate::error::SpvResult;
use tw_utxo::encode::compact_integer::CompactInteger;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

/// A `headers` message containing a list of block headers.
///
/// Each header on the wire is followed by a varint `tx_count` (always 0).
#[derive(Debug, Clone)]
pub struct HeadersMessage {
    pub headers: Vec<BlockHeader>,
}

impl HeadersMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        CompactInteger::from(self.headers.len()).encode(&mut stream);
        for header in &self.headers {
            header.encode(&mut stream);
            // tx_count is always 0
            CompactInteger::from(0usize).encode(&mut stream);
        }
        stream.out()
    }

    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let count = reader.read_compact_int()? as usize;
        let mut headers = Vec::with_capacity(count);
        for _ in 0..count {
            let header = BlockHeader::decode(&mut reader)?;
            // Read and discard the tx_count varint.
            let _tx_count = reader.read_compact_int()?;
            headers.push(header);
        }
        Ok(HeadersMessage { headers })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tw_hash::H256;

    fn sample_header(nonce: u32) -> BlockHeader {
        BlockHeader {
            version: 0x20000000,
            prev_block: H256::from([0xAA; 32]),
            merkle_root: H256::from([0xBB; 32]),
            timestamp: 1231006505,
            target: 0x1d00ffff,
            nonce,
        }
    }

    #[test]
    fn test_headers_message_roundtrip() {
        let msg = HeadersMessage {
            headers: vec![sample_header(1), sample_header(2)],
        };
        let serialized = msg.serialize();
        let parsed = HeadersMessage::parse(&serialized).unwrap();
        assert_eq!(parsed.headers.len(), 2);
        assert_eq!(parsed.headers[0].nonce, 1);
        assert_eq!(parsed.headers[1].nonce, 2);
        assert_eq!(parsed.headers[0].version, msg.headers[0].version);
        assert_eq!(parsed.headers[1].timestamp, msg.headers[1].timestamp);
    }

    #[test]
    fn test_headers_message_empty() {
        let msg = HeadersMessage {
            headers: Vec::new(),
        };
        let serialized = msg.serialize();
        let parsed = HeadersMessage::parse(&serialized).unwrap();
        assert!(parsed.headers.is_empty());
    }
}
