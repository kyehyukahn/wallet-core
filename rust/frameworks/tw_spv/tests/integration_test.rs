// SPDX-License-Identifier: Apache-2.0
//
// End-to-end integration test verifying all tw_spv components work together.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tw_hash::H256;
use tw_spv::bitcoin_chain::BitcoinMainnet;
use tw_spv::bloom_filter::{BloomFilter, UpdateType};
use tw_spv::chain::{BlockHeader, SpvChain};
use tw_spv::events::{SyncCapability, SyncEvent};
use tw_spv::merkle_block::MerkleBlock;
use tw_spv::messages::feefilter::FeeFilterMessage;
use tw_spv::messages::header::MessageHeader;
use tw_spv::messages::inv::{InvMessage, InvType, InvVector};
use tw_spv::messages::ping::PingMessage;
use tw_spv::messages::version::VersionMessage;
use tw_spv::peer::{
    PeerConfig, PeerInfo, PeerState, PeerStatus, SERVICES_NODE_BLOOM, SERVICES_NODE_NETWORK,
};
use tw_spv::peer_manager::PeerPool;
use tw_spv::sync_manager::SyncState;

/// Full SPV pipeline: chain config -> bloom filter -> messages -> peer handshake -> sync state machine
#[test]
fn test_full_spv_pipeline() {
    let chain = BitcoinMainnet;

    // 1. Chain config is valid
    assert_eq!(chain.network_magic(), 0xD9B4BEF9);
    assert!(chain.supports_bloom_filtering());
    assert!(!chain.checkpoints().is_empty());

    // 2. Build a bloom filter with wallet addresses
    let mut bloom = BloomFilter::new(0.0005, 100, 42, UpdateType::All);
    bloom.insert(b"address_external_0");
    bloom.insert(b"address_external_1");
    bloom.insert(b"address_internal_0");
    assert!(bloom.contains(b"address_external_0"));
    assert!(!bloom.contains(b"not_my_address"));

    // 3. Serialize bloom filter for filterload message
    let filter_payload = bloom.serialize();
    let parsed_bloom = BloomFilter::parse(&filter_payload).unwrap();
    assert!(parsed_bloom.contains(b"address_external_0"));

    // 4. Create and frame a version message
    let peer_config = PeerConfig {
        address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(198, 51, 100, 1), 8333)),
        magic_number: chain.network_magic(),
        protocol_version: chain.protocol_version(),
        min_protocol_version: chain.min_protocol_version(),
        user_agent: "/rwallet:1.0/".to_string(),
        start_height: 800000,
        earliest_key_time: 1700000000,
    };
    let mut peer = PeerState::new(peer_config);

    // 5. Build version message, verify it round-trips
    let version_msg = peer.build_version_message();
    let version_payload = version_msg.serialize();
    let parsed_version = VersionMessage::parse(&version_payload).unwrap();
    assert_eq!(parsed_version.version, 70013);
    assert_eq!(parsed_version.user_agent, "/rwallet:1.0/");
    assert_eq!(parsed_version.start_height, 800000);
    assert!(!parsed_version.relay); // SPV: relay=false

    // 6. Frame the version message with header
    let framed = peer.frame_message("version", &version_payload);
    let header = MessageHeader::parse(&framed[..24]).unwrap();
    assert_eq!(header.magic, 0xD9B4BEF9);
    assert_eq!(header.command, "version");
    assert!(header.verify_checksum(&framed[24..]).is_ok());

    // 7. Simulate receiving a version from remote peer
    let remote_version = VersionMessage {
        version: 70015,
        services: SERVICES_NODE_NETWORK | SERVICES_NODE_BLOOM,
        timestamp: 1700000000,
        recv_services: 0,
        recv_addr: [0u8; 16],
        recv_port: 0,
        from_services: SERVICES_NODE_NETWORK | SERVICES_NODE_BLOOM,
        from_addr: [0u8; 16],
        from_port: 8333,
        nonce: 99999,
        user_agent: "/Satoshi:25.0.0/".to_string(),
        start_height: 820000,
        relay: true,
    };
    peer.version_sent = true;
    peer.handle_version(&remote_version).unwrap();
    assert!(peer.supports_bloom());

    // 8. Receive verack -> handshake complete
    peer.handle_verack();
    assert!(peer.is_handshake_complete());
    assert_eq!(peer.status, PeerStatus::Connected);

    // 9. Test message roundtrips (inv, ping, feefilter)
    let inv = InvMessage {
        inventory: vec![InvVector {
            inv_type: InvType::FilteredWitnessBlock,
            hash: H256::from([0xAA; 32]),
        }],
    };
    let inv_data = inv.serialize();
    let parsed_inv = InvMessage::parse(&inv_data).unwrap();
    assert_eq!(
        parsed_inv.inventory[0].inv_type,
        InvType::FilteredWitnessBlock
    );

    let ping = PingMessage { nonce: 0xDEADBEEF };
    let ping_data = ping.serialize();
    let parsed_ping = PingMessage::parse(&ping_data).unwrap();
    assert_eq!(parsed_ping.nonce, 0xDEADBEEF);

    let fee = FeeFilterMessage { fee_rate: 5000 };
    let fee_data = fee.serialize();
    let parsed_fee = FeeFilterMessage::parse(&fee_data).unwrap();
    assert_eq!(parsed_fee.fee_rate, 5000);

    // 10. PeerPool: connect peers, verify capability transitions
    let mut pool = PeerPool::new();
    assert_eq!(pool.capability(), SyncCapability::Unsupported);

    let peer_info = PeerInfo {
        address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(198, 51, 100, 1), 8333)),
        services: SERVICES_NODE_NETWORK | SERVICES_NODE_BLOOM,
        timestamp: 1700000000,
    };
    let events = pool.on_peer_connected(peer_info, 820000, true);
    assert_eq!(pool.capability(), SyncCapability::FullBip37);
    assert!(events
        .iter()
        .any(|e| matches!(e, SyncEvent::PeerConnected(_))));
    assert!(pool.download_peer().is_some());

    // 11. SyncManager: start sync, process headers
    let mut sync = SyncState::new(&chain, 1700000000);
    let events = sync.start_sync();
    assert!(events
        .iter()
        .any(|e| matches!(e, SyncEvent::SyncStarted { .. })));

    // Simulate receiving 5 headers
    let headers: Vec<BlockHeader> = (0..5)
        .map(|i| BlockHeader {
            version: 0x20000000,
            prev_block: H256::from([i as u8; 32]),
            merkle_root: H256::from([(i + 50) as u8; 32]),
            timestamp: 1700000000 + i * 600,
            target: 0x1d00ffff,
            nonce: i,
        })
        .collect();
    let (events, _need_more) = sync.process_headers(headers, 820000);
    assert!(events
        .iter()
        .any(|e| matches!(e, SyncEvent::HeadersProgress { .. })));
    assert_eq!(sync.last_height(), 5);

    // 12. MerkleBlock: verify a single-tx block via wire-format parse
    let tx_hash = H256::from([0xCC; 32]);
    let merkle_header = BlockHeader {
        version: 1,
        prev_block: H256::from([0x00; 32]),
        merkle_root: tx_hash,
        timestamp: 1700000000,
        target: 0x1d00ffff,
        nonce: 0,
    };

    // Build the wire format: header + total_tx(u32) + compact_int(hash_count)
    // + hash + compact_int(flag_len) + flags
    let mut merkle_data = tw_utxo::encode::encode(&merkle_header);
    // total_tx = 1 (LE u32)
    merkle_data.extend_from_slice(&1u32.to_le_bytes());
    // hash_count = 1 (compact int, single byte)
    merkle_data.push(1u8);
    // the tx hash
    merkle_data.extend_from_slice(tx_hash.as_slice());
    // flag_len = 1 (compact int)
    merkle_data.push(1u8);
    // flags: bit 0 = 1 (matched leaf)
    merkle_data.push(0x01);

    let merkle_block = MerkleBlock::parse(&merkle_data).unwrap();
    let matched = merkle_block.matched_tx_hashes().unwrap();
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0], tx_hash);
}

/// Verify all chain configs are distinct and properly configured
#[test]
fn test_multi_chain_configs_distinct() {
    use tw_spv::bcash_chain::BitcoinCashMainnet;
    use tw_spv::dogecoin_chain::DogecoinMainnet;
    use tw_spv::litecoin_chain::LitecoinMainnet;

    let btc = BitcoinMainnet;
    let bch = BitcoinCashMainnet;
    let ltc = LitecoinMainnet;
    let doge = DogecoinMainnet;

    // All have unique magic numbers
    let magics = [
        btc.network_magic(),
        bch.network_magic(),
        ltc.network_magic(),
        doge.network_magic(),
    ];
    for i in 0..magics.len() {
        for j in (i + 1)..magics.len() {
            assert_ne!(
                magics[i], magics[j],
                "chains {} and {} have same magic",
                i, j
            );
        }
    }

    // All have unique chain IDs
    assert_ne!(btc.chain_id(), bch.chain_id());
    assert_ne!(btc.chain_id(), ltc.chain_id());
    assert_ne!(btc.chain_id(), doge.chain_id());
    assert_ne!(bch.chain_id(), ltc.chain_id());

    // All have unique ports (except BCash which shares 8333 with Bitcoin)
    assert_ne!(btc.default_port(), ltc.default_port());
    assert_ne!(btc.default_port(), doge.default_port());
    assert_ne!(ltc.default_port(), doge.default_port());

    // All support bloom filtering
    assert!(btc.supports_bloom_filtering());
    assert!(bch.supports_bloom_filtering());
    assert!(ltc.supports_bloom_filtering());
    assert!(doge.supports_bloom_filtering());

    // All have checkpoints
    assert!(!btc.checkpoints().is_empty());
    assert!(!bch.checkpoints().is_empty());
    assert!(!ltc.checkpoints().is_empty());
    assert!(!doge.checkpoints().is_empty());
}
