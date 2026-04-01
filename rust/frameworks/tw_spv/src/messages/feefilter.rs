// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::SpvResult;
use tw_utxo::encode::stream::Stream;

/// A `feefilter` message (BIP133) indicating the minimum fee rate
/// (in satoshis per kilobyte) a peer will accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeFilterMessage {
    pub fee_rate: u64,
}

impl FeeFilterMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        stream.append(&self.fee_rate);
        stream.out()
    }

    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let fee_rate = reader.read_u64_le()?;
        Ok(FeeFilterMessage { fee_rate })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feefilter_roundtrip() {
        let msg = FeeFilterMessage { fee_rate: 48508 };
        let serialized = msg.serialize();
        let parsed = FeeFilterMessage::parse(&serialized).unwrap();
        assert_eq!(parsed, msg);
    }
}
