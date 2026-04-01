// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::{SpvError, SpvResult};

/// Reject code: the transaction/block is invalid.
pub const REJECT_INVALID: u8 = 0x10;

/// Reject code: an input is already spent.
pub const REJECT_SPENT: u8 = 0x12;

/// Reject code: not standard (policy violation).
pub const REJECT_NONSTANDARD: u8 = 0x40;

/// Reject code: output is below dust threshold.
pub const REJECT_DUST: u8 = 0x41;

/// Reject code: fee is too low.
pub const REJECT_LOWFEE: u8 = 0x42;

/// A `reject` message received from a peer indicating that a previously
/// sent message was rejected.
///
/// This message is only parsed, never sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectMessage {
    pub message: String,
    pub code: u8,
    pub reason: String,
    pub data: Vec<u8>,
}

impl RejectMessage {
    /// Parses a `reject` message from raw bytes.
    ///
    /// Wire format:
    ///   - var_str: rejected message type (e.g. "tx", "block")
    ///   - u8: reject code
    ///   - var_str: human-readable reason
    ///   - remaining bytes: extra data (e.g. tx/block hash)
    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);

        let msg_len = reader.read_compact_int()? as usize;
        let msg_bytes = reader.read_bytes(msg_len)?;
        let message = String::from_utf8(msg_bytes)
            .map_err(|e| SpvError::InvalidData(format!("invalid message UTF-8: {}", e)))?;

        let code = reader.read_u8()?;

        let reason_len = reader.read_compact_int()? as usize;
        let reason_bytes = reader.read_bytes(reason_len)?;
        let reason = String::from_utf8(reason_bytes)
            .map_err(|e| SpvError::InvalidData(format!("invalid reason UTF-8: {}", e)))?;

        let remaining = reader.remaining();
        let extra_data = if remaining > 0 {
            reader.read_bytes(remaining)?
        } else {
            Vec::new()
        };

        Ok(RejectMessage {
            message,
            code,
            reason,
            data: extra_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_parse() {
        // Manually construct: var_str("tx") + code(0x10) + var_str("invalid") + 32 bytes data
        let mut bytes = Vec::new();
        // message = "tx"
        bytes.push(2u8);
        bytes.extend_from_slice(b"tx");
        // code
        bytes.push(REJECT_INVALID);
        // reason = "invalid"
        bytes.push(7u8);
        bytes.extend_from_slice(b"invalid");
        // extra data: 32-byte hash
        let hash = [0x42u8; 32];
        bytes.extend_from_slice(&hash);

        let msg = RejectMessage::parse(&bytes).unwrap();
        assert_eq!(msg.message, "tx");
        assert_eq!(msg.code, REJECT_INVALID);
        assert_eq!(msg.reason, "invalid");
        assert_eq!(msg.data, hash.to_vec());
    }
}
