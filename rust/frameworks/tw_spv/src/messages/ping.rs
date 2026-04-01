// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::SpvResult;
use tw_utxo::encode::stream::Stream;

/// A `ping` message containing a random nonce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingMessage {
    pub nonce: u64,
}

impl PingMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        stream.append(&self.nonce);
        stream.out()
    }

    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let nonce = reader.read_u64_le()?;
        Ok(PingMessage { nonce })
    }
}

/// A `pong` message has the same format as a `ping` message.
pub type PongMessage = PingMessage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_roundtrip() {
        let msg = PingMessage {
            nonce: 0xDEADBEEFCAFEBABE,
        };
        let serialized = msg.serialize();
        let parsed = PingMessage::parse(&serialized).unwrap();
        assert_eq!(parsed, msg);
    }
}
