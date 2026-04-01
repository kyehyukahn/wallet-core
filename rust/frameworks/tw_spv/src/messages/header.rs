// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::{SpvError, SpvResult};
use tw_hash::sha2::sha256_d;
use tw_utxo::encode::stream::Stream;

/// Size of a Bitcoin P2P message header in bytes.
pub const MESSAGE_HEADER_SIZE: usize = 24;

/// Maximum allowed message payload length (32 MiB).
pub const MAX_MSG_LENGTH: usize = 0x02000000;

/// The 24-byte Bitcoin P2P message header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHeader {
    /// Network magic bytes (e.g., 0xD9B4BEF9 for mainnet).
    pub magic: u32,
    /// Command name, up to 12 ASCII characters.
    pub command: String,
    /// Length of the payload in bytes.
    pub payload_length: u32,
    /// First 4 bytes of SHA256d(payload).
    pub checksum: [u8; 4],
}

impl MessageHeader {
    /// Creates a new `MessageHeader`, computing the checksum from the given payload.
    pub fn new(magic: u32, command: String, payload: &[u8]) -> Self {
        let hash = sha256_d(payload);
        let mut checksum = [0u8; 4];
        checksum.copy_from_slice(&hash[..4]);
        MessageHeader {
            magic,
            command,
            payload_length: payload.len() as u32,
            checksum,
        }
    }

    /// Serializes the header to exactly 24 bytes.
    ///
    /// Layout: 4 bytes magic (LE) + 12 bytes command (null-padded) + 4 bytes length (LE) + 4 bytes checksum.
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        stream.append(&self.magic);

        // Command: 12 bytes, null-padded.
        let mut cmd_bytes = [0u8; 12];
        let cmd = self.command.as_bytes();
        let copy_len = cmd.len().min(12);
        cmd_bytes[..copy_len].copy_from_slice(&cmd[..copy_len]);
        stream.append_raw_slice(&cmd_bytes);

        stream.append(&self.payload_length);
        stream.append_raw_slice(&self.checksum);

        stream.out()
    }

    /// Parses a `MessageHeader` from exactly 24 bytes.
    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        if data.len() < MESSAGE_HEADER_SIZE {
            return Err(SpvError::UnexpectedEof);
        }
        let mut reader = Reader::new(data);
        let magic = reader.read_u32_le()?;
        let command = reader.read_fixed_string(12)?;
        let payload_length = reader.read_u32_le()?;
        let checksum_bytes = reader.read_bytes(4)?;
        let mut checksum = [0u8; 4];
        checksum.copy_from_slice(&checksum_bytes);

        if payload_length as usize > MAX_MSG_LENGTH {
            return Err(SpvError::MessageTooLarge);
        }

        Ok(MessageHeader {
            magic,
            command,
            payload_length,
            checksum,
        })
    }

    /// Verifies that the checksum matches SHA256d of the given payload.
    pub fn verify_checksum(&self, payload: &[u8]) -> SpvResult<()> {
        let hash = sha256_d(payload);
        if self.checksum[..] != hash[..4] {
            return Err(SpvError::ChecksumMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_header_roundtrip() {
        let payload = b"hello world";
        let header = MessageHeader::new(0xD9B4BEF9, "version".to_string(), payload);

        assert_eq!(header.magic, 0xD9B4BEF9);
        assert_eq!(header.command, "version");
        assert_eq!(header.payload_length, payload.len() as u32);

        let serialized = header.serialize();
        assert_eq!(serialized.len(), MESSAGE_HEADER_SIZE);

        let parsed = MessageHeader::parse(&serialized).unwrap();
        assert_eq!(parsed.magic, header.magic);
        assert_eq!(parsed.command, header.command);
        assert_eq!(parsed.payload_length, header.payload_length);
        assert_eq!(parsed.checksum, header.checksum);
    }

    #[test]
    fn test_message_header_empty_payload() {
        let header = MessageHeader::new(0xD9B4BEF9, "verack".to_string(), &[]);
        assert_eq!(header.payload_length, 0);

        let serialized = header.serialize();
        assert_eq!(serialized.len(), MESSAGE_HEADER_SIZE);

        let parsed = MessageHeader::parse(&serialized).unwrap();
        assert_eq!(parsed.payload_length, 0);
        assert_eq!(parsed.command, "verack");

        // Verify checksum against empty payload.
        assert!(parsed.verify_checksum(&[]).is_ok());
    }

    #[test]
    fn test_message_header_checksum_mismatch() {
        let payload = b"correct payload";
        let header = MessageHeader::new(0xD9B4BEF9, "tx".to_string(), payload);

        // Verify with wrong payload should fail.
        let wrong_payload = b"wrong payload";
        assert_eq!(
            header.verify_checksum(wrong_payload),
            Err(SpvError::ChecksumMismatch)
        );

        // Verify with correct payload should succeed.
        assert!(header.verify_checksum(payload).is_ok());
    }
}
