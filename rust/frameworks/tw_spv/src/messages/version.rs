// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::{SpvError, SpvResult};
use tw_utxo::encode::compact_integer::CompactInteger;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

/// Bitcoin P2P version message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMessage {
    pub version: u32,
    pub services: u64,
    pub timestamp: i64,
    pub recv_services: u64,
    pub recv_addr: [u8; 16],
    pub recv_port: u16,
    pub from_services: u64,
    pub from_addr: [u8; 16],
    pub from_port: u16,
    pub nonce: u64,
    pub user_agent: String,
    pub start_height: u32,
    pub relay: bool,
}

impl VersionMessage {
    /// Serializes the version message to bytes.
    ///
    /// All integer fields are little-endian, except ports which are big-endian.
    /// The user_agent is encoded as a var_str (CompactInteger length prefix + bytes).
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        stream.append(&self.version);
        stream.append(&self.services);
        stream.append(&self.timestamp);
        // recv: services + 16-byte addr + 2-byte port (BE)
        stream.append(&self.recv_services);
        stream.append_raw_slice(&self.recv_addr);
        stream.append_raw_slice(&self.recv_port.to_be_bytes());
        // from: services + 16-byte addr + 2-byte port (BE)
        stream.append(&self.from_services);
        stream.append_raw_slice(&self.from_addr);
        stream.append_raw_slice(&self.from_port.to_be_bytes());
        // nonce
        stream.append(&self.nonce);
        // user_agent as var_str
        let ua_bytes = self.user_agent.as_bytes();
        CompactInteger::from(ua_bytes.len()).encode(&mut stream);
        stream.append_raw_slice(ua_bytes);
        // start_height
        stream.append(&self.start_height);
        // relay
        stream.append(&(self.relay as u8));

        stream.out()
    }

    /// Parses a version message from bytes.
    ///
    /// If the relay byte is missing (pre-BIP37 peers), it defaults to `true`.
    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);

        let version = reader.read_u32_le()?;
        let services = reader.read_u64_le()?;
        let timestamp = reader.read_i64_le()?;

        // recv addr
        let recv_services = reader.read_u64_le()?;
        let recv_addr_bytes = reader.read_bytes(16)?;
        let mut recv_addr = [0u8; 16];
        recv_addr.copy_from_slice(&recv_addr_bytes);
        let recv_port_bytes = reader.read_bytes(2)?;
        let recv_port = u16::from_be_bytes([recv_port_bytes[0], recv_port_bytes[1]]);

        // from addr
        let from_services = reader.read_u64_le()?;
        let from_addr_bytes = reader.read_bytes(16)?;
        let mut from_addr = [0u8; 16];
        from_addr.copy_from_slice(&from_addr_bytes);
        let from_port_bytes = reader.read_bytes(2)?;
        let from_port = u16::from_be_bytes([from_port_bytes[0], from_port_bytes[1]]);

        let nonce = reader.read_u64_le()?;

        // user_agent as var_str
        let ua_len = reader.read_compact_int()? as usize;
        let ua_bytes = reader.read_bytes(ua_len)?;
        let user_agent = String::from_utf8(ua_bytes)
            .map_err(|e| SpvError::InvalidData(format!("invalid user_agent UTF-8: {}", e)))?;

        let start_height = reader.read_u32_le()?;

        // BIP37: relay field is optional; if missing, default to true.
        let relay = if reader.remaining() > 0 {
            reader.read_u8()? != 0
        } else {
            true
        };

        Ok(VersionMessage {
            version,
            services,
            timestamp,
            recv_services,
            recv_addr,
            recv_port,
            from_services,
            from_addr,
            from_port,
            nonce,
            user_agent,
            start_height,
            relay,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_version() -> VersionMessage {
        let mut recv_addr = [0u8; 16];
        recv_addr[10] = 0xFF;
        recv_addr[11] = 0xFF;
        recv_addr[12] = 127;
        recv_addr[15] = 1;

        let mut from_addr = [0u8; 16];
        from_addr[10] = 0xFF;
        from_addr[11] = 0xFF;
        from_addr[12] = 192;
        from_addr[13] = 168;
        from_addr[14] = 1;
        from_addr[15] = 100;

        VersionMessage {
            version: 70015,
            services: 1,
            timestamp: 1_700_000_000,
            recv_services: 1,
            recv_addr,
            recv_port: 8333,
            from_services: 1,
            from_addr,
            from_port: 8333,
            nonce: 0xDEADBEEFCAFEBABE,
            user_agent: "/TrustWallet:0.1/".to_string(),
            start_height: 800_000,
            relay: true,
        }
    }

    #[test]
    fn test_version_message_roundtrip() {
        let msg = sample_version();
        let serialized = msg.serialize();
        let parsed = VersionMessage::parse(&serialized).unwrap();

        assert_eq!(parsed.version, msg.version);
        assert_eq!(parsed.services, msg.services);
        assert_eq!(parsed.timestamp, msg.timestamp);
        assert_eq!(parsed.recv_services, msg.recv_services);
        assert_eq!(parsed.recv_addr, msg.recv_addr);
        assert_eq!(parsed.recv_port, msg.recv_port);
        assert_eq!(parsed.from_services, msg.from_services);
        assert_eq!(parsed.from_addr, msg.from_addr);
        assert_eq!(parsed.from_port, msg.from_port);
        assert_eq!(parsed.nonce, msg.nonce);
        assert_eq!(parsed.user_agent, msg.user_agent);
        assert_eq!(parsed.start_height, msg.start_height);
        assert_eq!(parsed.relay, msg.relay);
    }

    #[test]
    fn test_version_message_no_relay_field() {
        let msg = sample_version();
        let serialized = msg.serialize();

        // Truncate the last byte (relay field) to simulate pre-BIP37 peer.
        let truncated = &serialized[..serialized.len() - 1];
        let parsed = VersionMessage::parse(truncated).unwrap();

        // All fields should match except relay defaults to true.
        assert_eq!(parsed.version, msg.version);
        assert_eq!(parsed.user_agent, msg.user_agent);
        assert_eq!(parsed.start_height, msg.start_height);
        assert!(parsed.relay); // defaults to true
    }
}
