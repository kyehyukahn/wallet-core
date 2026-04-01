// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::error::SpvResult;
use crate::merkle_block::MerkleBlock;

/// Thin wrapper around `MerkleBlock` for the `merkleblock` P2P message.
pub struct MerkleBlockMessage;

impl MerkleBlockMessage {
    /// Parses a `merkleblock` message payload by delegating to `MerkleBlock::parse`.
    pub fn parse(data: &[u8]) -> SpvResult<MerkleBlock> {
        MerkleBlock::parse(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::BlockHeader;
    use tw_hash::H256;

    #[test]
    fn test_merkleblock_msg_parse() {
        // Build a minimal merkle block: header + total_tx + 1 hash + flags
        let tx_hash = H256::from([0x42; 32]);
        let header = BlockHeader {
            version: 1,
            prev_block: H256::from([0u8; 32]),
            merkle_root: tx_hash,
            timestamp: 1231006505,
            target: 0x1d00ffff,
            nonce: 0,
        };

        let mut data = tw_utxo::encode::encode(&header);
        // total_tx = 1
        data.extend_from_slice(&1u32.to_le_bytes());
        // hash_count = 1
        data.push(1u8);
        // the tx hash
        data.extend_from_slice(tx_hash.as_slice());
        // flag_len = 1, flags = 0x01
        data.push(1u8);
        data.push(0x01);

        let mb = MerkleBlockMessage::parse(&data).unwrap();
        assert_eq!(mb.total_tx, 1);
        assert_eq!(mb.hashes.len(), 1);
        assert_eq!(mb.hashes[0], tx_hash);
    }
}
