// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::error::{SpvError, SpvResult};
use crate::messages::header::MessageHeader;
use crate::messages::version::VersionMessage;
use std::net::SocketAddr;
use std::time::Duration;
use tw_hash::sha2::sha256;

// Service bit constants.
pub const SERVICES_NODE_NETWORK: u64 = 0x01;
pub const SERVICES_NODE_BLOOM: u64 = 0x04;
pub const SERVICES_NODE_WITNESS: u64 = 0x08;
pub const SERVICES_NODE_COMPACT_FILTERS: u64 = 0x40;

/// Connection status of a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    Disconnected,
    Connecting,
    Connected,
}

/// Information about a peer on the network.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub address: SocketAddr,
    pub services: u64,
    pub timestamp: u64,
}

impl PeerInfo {
    /// Returns `true` if the peer advertises the NODE_BLOOM service bit.
    pub fn supports_bloom(&self) -> bool {
        self.services & SERVICES_NODE_BLOOM != 0
    }

    /// Returns `true` if the peer advertises the NODE_WITNESS service bit.
    pub fn supports_witness(&self) -> bool {
        self.services & SERVICES_NODE_WITNESS != 0
    }
}

/// Configuration for connecting to a peer.
#[derive(Debug, Clone)]
pub struct PeerConfig {
    pub address: SocketAddr,
    pub magic_number: u32,
    pub protocol_version: u32,
    pub min_protocol_version: u32,
    pub user_agent: String,
    pub start_height: u32,
    pub earliest_key_time: u64,
}

/// Manages handshake state without owning TCP connections.
/// Operates on raw byte buffers.
pub struct PeerState {
    pub config: PeerConfig,
    pub status: PeerStatus,
    pub remote_version: u32,
    pub remote_user_agent: String,
    pub remote_start_height: u32,
    pub remote_services: u64,
    pub fee_per_kb: u64,
    pub nonce: u64,
    pub needs_filter_update: bool,
    pub version_sent: bool,
    pub version_ack_received: bool,
    pub version_received: bool,
    pub ping_nonce: u64,
    pub ping_time: Duration,
}

impl PeerState {
    /// Creates a new `PeerState` with the given configuration.
    /// The nonce is derived deterministically from the address hash.
    pub fn new(config: PeerConfig) -> Self {
        let addr_str = config.address.to_string();
        let hash = sha256(addr_str.as_bytes());
        let nonce = u64::from_le_bytes(hash[..8].try_into().unwrap());

        PeerState {
            config,
            status: PeerStatus::Disconnected,
            remote_version: 0,
            remote_user_agent: String::new(),
            remote_start_height: 0,
            remote_services: 0,
            fee_per_kb: 0,
            nonce,
            needs_filter_update: false,
            version_sent: false,
            version_ack_received: false,
            version_received: false,
            ping_nonce: 0,
            ping_time: Duration::ZERO,
        }
    }

    /// Builds a version message payload for this peer.
    /// Uses services=0 (SPV node) and relay=false (BIP37).
    pub fn build_version_message(&self) -> VersionMessage {
        // Map the peer's address to a 16-byte IPv6-mapped IPv4 address.
        let (recv_addr, recv_port) = addr_to_net(&self.config.address);

        VersionMessage {
            version: self.config.protocol_version,
            services: 0, // SPV node advertises no services
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs() as i64,
            recv_services: 0,
            recv_addr,
            recv_port,
            from_services: 0,
            from_addr: [0u8; 16],
            from_port: 0,
            nonce: self.nonce,
            user_agent: self.config.user_agent.clone(),
            start_height: self.config.start_height,
            relay: false, // BIP37: don't relay until filter is set
        }
    }

    /// Handles an incoming version message from the remote peer.
    /// Rejects if the remote protocol version is below `min_protocol_version`.
    pub fn handle_version(&mut self, msg: &VersionMessage) -> SpvResult<()> {
        if msg.version < self.config.min_protocol_version {
            return Err(SpvError::InvalidData(format!(
                "peer protocol version {} below minimum {}",
                msg.version, self.config.min_protocol_version
            )));
        }

        self.remote_version = msg.version;
        self.remote_user_agent = msg.user_agent.clone();
        self.remote_start_height = msg.start_height;
        self.remote_services = msg.services;
        self.version_received = true;

        if self.version_ack_received {
            self.status = PeerStatus::Connected;
        }

        Ok(())
    }

    /// Handles an incoming verack message.
    /// Sets status to Connected if the version message has also been received.
    pub fn handle_verack(&mut self) {
        self.version_ack_received = true;
        if self.version_received {
            self.status = PeerStatus::Connected;
        }
    }

    /// Returns `true` if the handshake is complete.
    pub fn is_handshake_complete(&self) -> bool {
        self.version_sent && self.version_received && self.version_ack_received
    }

    /// Returns `true` if the remote peer supports the NODE_BLOOM service.
    pub fn supports_bloom(&self) -> bool {
        self.remote_services & SERVICES_NODE_BLOOM != 0
    }

    /// Wraps a payload with a `MessageHeader` to produce a framed message.
    pub fn frame_message(&self, command: &str, payload: &[u8]) -> Vec<u8> {
        let header = MessageHeader::new(self.config.magic_number, command.to_string(), payload);
        let mut result = header.serialize();
        result.extend_from_slice(payload);
        result
    }
}

/// Converts a `SocketAddr` to a 16-byte IPv6-mapped address and port.
fn addr_to_net(addr: &SocketAddr) -> ([u8; 16], u16) {
    let port = addr.port();
    let ip_bytes = match addr.ip() {
        std::net::IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            let mut mapped = [0u8; 16];
            mapped[10] = 0xFF;
            mapped[11] = 0xFF;
            mapped[12] = octets[0];
            mapped[13] = octets[1];
            mapped[14] = octets[2];
            mapped[15] = octets[3];
            mapped
        },
        std::net::IpAddr::V6(ipv6) => ipv6.octets(),
    };
    (ip_bytes, port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::header::MESSAGE_HEADER_SIZE;

    fn test_config() -> PeerConfig {
        PeerConfig {
            address: "127.0.0.1:8333".parse().unwrap(),
            magic_number: 0xD9B4BEF9,
            protocol_version: 70015,
            min_protocol_version: 70012,
            user_agent: "/TrustWallet:0.1/".to_string(),
            start_height: 800_000,
            earliest_key_time: 1_700_000_000,
        }
    }

    fn remote_version_msg(version: u32, services: u64) -> VersionMessage {
        VersionMessage {
            version,
            services,
            timestamp: 1_700_000_000,
            recv_services: 0,
            recv_addr: [0u8; 16],
            recv_port: 0,
            from_services: services,
            from_addr: [0u8; 16],
            from_port: 8333,
            nonce: 0x1234,
            user_agent: "/RemotePeer:1.0/".to_string(),
            start_height: 810_000,
            relay: true,
        }
    }

    #[test]
    fn test_peer_state_initial() {
        let state = PeerState::new(test_config());
        assert_eq!(state.status, PeerStatus::Disconnected);
        assert!(!state.is_handshake_complete());
    }

    #[test]
    fn test_peer_version_message_build_and_parse() {
        let state = PeerState::new(test_config());
        let msg = state.build_version_message();
        let serialized = msg.serialize();
        let parsed = VersionMessage::parse(&serialized).unwrap();

        assert_eq!(parsed.version, 70015);
        assert_eq!(parsed.services, 0); // SPV
        assert_eq!(parsed.user_agent, "/TrustWallet:0.1/");
        assert_eq!(parsed.start_height, 800_000);
        assert!(!parsed.relay); // BIP37
        assert_eq!(parsed.nonce, state.nonce);
    }

    #[test]
    fn test_peer_handshake_flow() {
        let mut state = PeerState::new(test_config());

        // Mark version as sent.
        state.version_sent = true;
        state.status = PeerStatus::Connecting;

        // Receive remote version.
        let remote = remote_version_msg(70015, SERVICES_NODE_NETWORK | SERVICES_NODE_BLOOM);
        state.handle_version(&remote).unwrap();
        assert_eq!(state.remote_version, 70015);
        assert_eq!(state.remote_user_agent, "/RemotePeer:1.0/");
        assert!(state.version_received);
        // Not yet connected — still waiting for verack.
        assert_ne!(state.status, PeerStatus::Connected);

        // Receive verack.
        state.handle_verack();
        assert_eq!(state.status, PeerStatus::Connected);
        assert!(state.is_handshake_complete());
    }

    #[test]
    fn test_peer_rejects_old_version() {
        let mut state = PeerState::new(test_config());
        let old_remote = remote_version_msg(70011, SERVICES_NODE_NETWORK);
        let result = state.handle_version(&old_remote);
        assert!(result.is_err());
        assert!(!state.version_received);
    }

    #[test]
    fn test_peer_frame_message() {
        let state = PeerState::new(test_config());
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
        let framed = state.frame_message("ping", &payload);

        // Total size = 24-byte header + 8-byte payload.
        assert_eq!(framed.len(), MESSAGE_HEADER_SIZE + payload.len());

        // Parse the header back and verify.
        let header = MessageHeader::parse(&framed[..MESSAGE_HEADER_SIZE]).unwrap();
        assert_eq!(header.command, "ping");
        assert_eq!(header.payload_length as usize, payload.len());
        assert_eq!(header.magic, 0xD9B4BEF9);

        // Verify payload follows the header.
        assert_eq!(&framed[MESSAGE_HEADER_SIZE..], &payload[..]);
    }

    #[test]
    fn test_peer_no_bloom_without_service_bit() {
        let mut state = PeerState::new(test_config());
        // Remote peer with NODE_NETWORK only, no NODE_BLOOM.
        let remote = remote_version_msg(70015, SERVICES_NODE_NETWORK);
        state.handle_version(&remote).unwrap();
        assert!(!state.supports_bloom());
    }
}
