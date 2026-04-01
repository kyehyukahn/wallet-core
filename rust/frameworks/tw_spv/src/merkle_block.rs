// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.
//
// Portions of this file are derived from breadwallet-core
// (https://github.com/voisine/breadwallet-core),
// Copyright (c) 2015 breadwallet LLC, licensed under the MIT License.
// See LICENSE-3RD-PARTY.txt for the full license text.

use crate::chain::BlockHeader;
use crate::decode::{Decodable, Reader};
use crate::error::{SpvError, SpvResult};
use tw_hash::sha2::sha256_d;
use tw_hash::H256;

/// Maximum allowed clock drift for block timestamps (2 hours).
const MAX_TIME_DRIFT: u32 = 2 * 60 * 60;

/// A parsed `merkleblock` message containing a block header and a partial
/// merkle tree that proves inclusion of one or more transactions.
#[derive(Debug, Clone)]
pub struct MerkleBlock {
    pub header: BlockHeader,
    pub total_tx: u32,
    pub hashes: Vec<H256>,
    pub flags: Vec<u8>,
    pub height: u32,
}

impl MerkleBlock {
    /// Parse a `MerkleBlock` from raw wire bytes.
    ///
    /// If the data is exactly 80 bytes, it is treated as a header-only block
    /// (no merkle proof data).
    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let header = BlockHeader::decode(&mut reader)?;

        if reader.is_empty() {
            // Header-only block.
            return Ok(MerkleBlock {
                header,
                total_tx: 0,
                hashes: Vec::new(),
                flags: Vec::new(),
                height: 0,
            });
        }

        let total_tx = reader.read_u32_le()?;

        let hash_count = reader.read_compact_int()? as usize;
        let mut hashes = Vec::with_capacity(hash_count);
        for _ in 0..hash_count {
            hashes.push(reader.read_h256()?);
        }

        let flags = reader.read_var_bytes()?;

        Ok(MerkleBlock {
            header,
            total_tx,
            hashes,
            flags,
            height: 0,
        })
    }

    /// Returns the double-SHA256 hash of the block header.
    pub fn block_hash(&self) -> H256 {
        self.header.block_hash()
    }

    /// Walk the partial merkle tree and extract matched transaction hashes.
    ///
    /// The algorithm matches breadwallet-core `BRMerkleBlockTxHashes()`.
    pub fn matched_tx_hashes(&self) -> SpvResult<Vec<H256>> {
        if self.total_tx == 0 {
            return Ok(Vec::new());
        }

        let height = self.tree_height();
        let mut bit_idx: usize = 0;
        let mut hash_idx: usize = 0;
        let mut matched = Vec::new();

        let root = self.walk_tree(height, 0, &mut bit_idx, &mut hash_idx, &mut matched)?;

        if root != self.header.merkle_root {
            return Err(SpvError::InvalidData("merkle root mismatch".to_string()));
        }

        Ok(matched)
    }

    /// Validate the merkle block.
    ///
    /// Checks:
    /// 1. Timestamp is not more than 2 hours in the future.
    /// 2. Proof-of-work satisfies the declared target.
    /// 3. Merkle tree root matches the header (if partial tree present).
    pub fn is_valid(&self, current_time: u32) -> SpvResult<bool> {
        // Check timestamp drift.
        if self.header.timestamp > current_time + MAX_TIME_DRIFT {
            return Ok(false);
        }

        // Check proof-of-work.
        let target_u256 = Self::target_to_u256(self.header.target);
        let hash_u256 = Self::hash_to_u256(&self.header.block_hash());
        if hash_u256 > target_u256 {
            return Ok(false);
        }

        // If there is a partial merkle tree, verify it produces the correct root.
        if self.total_tx > 0 {
            let _ = self.matched_tx_hashes()?;
        }

        Ok(true)
    }

    /// Returns true if the given transaction hash is among the matched hashes
    /// in the partial merkle tree.
    pub fn contains_tx_hash(&self, tx_hash: &H256) -> SpvResult<bool> {
        let matched = self.matched_tx_hashes()?;
        Ok(matched.contains(tx_hash))
    }

    // --- Internal helpers ---

    /// Height of the merkle tree for `total_tx` leaves.
    fn tree_height(&self) -> usize {
        let mut h = 0u32;
        let mut n = self.total_tx;
        while n > 1 {
            n = n.div_ceil(2);
            h += 1;
        }
        h as usize
    }

    /// Width of the tree at a given `height` level (0 = root).
    fn tree_width(&self, height: usize) -> usize {
        let tree_h = self.tree_height();
        // At tree_height level we have total_tx leaves.
        // At level h from the top, width = (total_tx + (1 << (tree_h - h)) - 1) >> (tree_h - h)
        let shift = tree_h - height;
        ((self.total_tx as usize) + (1 << shift) - 1) >> shift
    }

    /// Read a single bit from the flags bitvector.
    fn read_flag_bit(flags: &[u8], bit_idx: &mut usize) -> bool {
        let idx = *bit_idx;
        *bit_idx += 1;
        if idx / 8 >= flags.len() {
            return false;
        }
        (flags[idx / 8] >> (idx % 8)) & 1 != 0
    }

    /// Recursively walk the partial merkle tree (breadwallet algorithm).
    ///
    /// `height` is the current level (0 = root, tree_height = leaves).
    /// `pos` is the node position at this level.
    fn walk_tree(
        &self,
        height: usize,
        pos: usize,
        bit_idx: &mut usize,
        hash_idx: &mut usize,
        matched: &mut Vec<H256>,
    ) -> SpvResult<H256> {
        let flag = Self::read_flag_bit(&self.flags, bit_idx);

        if height == 0 || !flag {
            // Leaf node or pruned subtree: consume a hash.
            if *hash_idx >= self.hashes.len() {
                return Err(SpvError::InvalidData(
                    "merkle tree hash index out of bounds".to_string(),
                ));
            }
            let hash = self.hashes[*hash_idx];
            *hash_idx += 1;

            if height == 0 && flag {
                // This is a matched leaf.
                matched.push(hash);
            }
            return Ok(hash);
        }

        // Internal node with flag=1: descend into children.
        let left = self.walk_tree(height - 1, pos * 2, bit_idx, hash_idx, matched)?;

        let right = if pos * 2 + 1 < self.tree_width(height - 1) {
            self.walk_tree(height - 1, pos * 2 + 1, bit_idx, hash_idx, matched)?
        } else {
            left
        };

        // Combine left and right hashes.
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(left.as_slice());
        combined[32..].copy_from_slice(right.as_slice());
        let parent_hash = sha256_d(&combined);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&parent_hash);
        Ok(H256::from(arr))
    }

    /// Convert a compact target (nBits) to a 256-bit big-endian value.
    fn target_to_u256(n_bits: u32) -> [u8; 32] {
        let mut target = [0u8; 32];
        let exponent = ((n_bits >> 24) & 0xFF) as usize;
        let mantissa = n_bits & 0x007F_FFFF;

        if exponent == 0 || exponent > 32 {
            return target;
        }

        // The mantissa occupies 3 bytes at position (exponent - 3) from the
        // least-significant end, stored big-endian in our array.
        // In a 32-byte big-endian array, byte index 0 is most significant.
        // The mantissa's MSB goes at index (32 - exponent).
        let start = 32usize.saturating_sub(exponent);
        if start < 32 {
            target[start] = ((mantissa >> 16) & 0xFF) as u8;
        }
        if start + 1 < 32 {
            target[start + 1] = ((mantissa >> 8) & 0xFF) as u8;
        }
        if start + 2 < 32 {
            target[start + 2] = (mantissa & 0xFF) as u8;
        }

        target
    }

    /// Convert an H256 hash to a big-endian 256-bit value for comparison.
    ///
    /// Bitcoin block hashes are stored in internal byte order (little-endian),
    /// so we reverse them for numeric comparison.
    fn hash_to_u256(hash: &H256) -> [u8; 32] {
        let mut result = [0u8; 32];
        let slice = hash.as_slice();
        for i in 0..32 {
            result[i] = slice[31 - i];
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_only_parse() {
        let header = BlockHeader {
            version: 1,
            prev_block: H256::from([0u8; 32]),
            merkle_root: H256::from([0u8; 32]),
            timestamp: 1231006505,
            target: 0x1d00ffff,
            nonce: 2083236893,
        };
        let encoded = tw_utxo::encode::encode(&header);
        assert_eq!(encoded.len(), 80);

        let mb = MerkleBlock::parse(&encoded).unwrap();
        assert_eq!(mb.total_tx, 0);
        assert!(mb.hashes.is_empty());
        assert!(mb.flags.is_empty());
    }

    #[test]
    fn test_target_to_u256_genesis() {
        // Genesis block target: 0x1d00ffff
        let target = MerkleBlock::target_to_u256(0x1d00ffff);
        // Expected: 00000000ffff00000000000000000000000000000000000000000000000000000
        // Exponent = 0x1d = 29, mantissa = 0x00ffff
        // Start index = 32 - 29 = 3
        assert_eq!(target[3], 0x00);
        assert_eq!(target[4], 0xff);
        assert_eq!(target[5], 0xff);
        // Everything before index 3 should be zero.
        assert_eq!(target[0], 0);
        assert_eq!(target[1], 0);
        assert_eq!(target[2], 0);
        // Everything after index 5 should be zero.
        for i in 6..32 {
            assert_eq!(target[i], 0, "byte {} should be zero", i);
        }
    }

    #[test]
    fn test_merkle_block_single_tx() {
        // Build a merkle block with a single transaction.
        // For 1 tx, the merkle root IS the tx hash.
        let tx_hash = H256::from([0x42; 32]);

        let header = BlockHeader {
            version: 1,
            prev_block: H256::from([0u8; 32]),
            merkle_root: tx_hash,
            timestamp: 1231006505,
            target: 0x1d00ffff,
            nonce: 0,
        };

        // Build the wire format: header + total_tx(u32) + compact_int(hash_count)
        // + hash + compact_int(flag_len) + flags
        let mut data = tw_utxo::encode::encode(&header);
        // total_tx = 1 (LE u32)
        data.extend_from_slice(&1u32.to_le_bytes());
        // hash_count = 1 (compact int, single byte)
        data.push(1u8);
        // the tx hash
        data.extend_from_slice(tx_hash.as_slice());
        // flag_len = 1 (compact int)
        data.push(1u8);
        // flags: bit 0 = 1 (matched leaf)
        data.push(0x01);

        let mb = MerkleBlock::parse(&data).unwrap();
        assert_eq!(mb.total_tx, 1);
        assert_eq!(mb.hashes.len(), 1);

        let matched = mb.matched_tx_hashes().unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0], tx_hash);

        assert!(mb.contains_tx_hash(&tx_hash).unwrap());
        assert!(!mb.contains_tx_hash(&H256::from([0x99; 32])).unwrap());
    }
}
