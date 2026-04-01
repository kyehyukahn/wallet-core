// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.
//
// Portions of this file are derived from breadwallet-core
// (https://github.com/voisine/breadwallet-core),
// Copyright (c) 2015 breadwallet LLC, licensed under the MIT License.
// See LICENSE-3RD-PARTY.txt for the full license text.

//! BIP37 Bloom Filter implementation.
//!
//! This module implements the Bloom filter as specified in BIP37,
//! matching the behavior of breadwallet-core's BRBloomFilter.c.

use crate::decode::Reader;
use crate::error::{SpvError, SpvResult};
use tw_utxo::encode::compact_integer::CompactInteger;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

/// Maximum filter length in bytes (BIP37).
const BLOOM_MAX_FILTER_LENGTH: usize = 36000;

/// Maximum number of hash functions (BIP37).
const BLOOM_MAX_HASH_FUNCS: u32 = 50;

/// Seed multiplier for BIP37 hash function indexing.
const MURMUR3_SEED_MULTIPLIER: u32 = 0xFBA4C795;

/// ln(2)
const LN2: f64 = core::f64::consts::LN_2;

/// ln(2)^2
const LN2_SQUARED: f64 = LN2 * LN2;

/// Describes how the filter should be updated when a match is found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    /// The filter is not adjusted when a match is found.
    None = 0,
    /// The outpoint is added to the filter if any data element in the
    /// output script is matched.
    All = 1,
    /// The outpoint is added only when the output script is a
    /// pay-to-pubkey or pay-to-multisig script.
    P2PubkeyOnly = 2,
}

impl TryFrom<u8> for UpdateType {
    type Error = SpvError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UpdateType::None),
            1 => Ok(UpdateType::All),
            2 => Ok(UpdateType::P2PubkeyOnly),
            _ => Err(SpvError::InvalidData(format!(
                "invalid bloom filter update type: {}",
                value
            ))),
        }
    }
}

/// A BIP37 Bloom filter for SPV clients.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    /// The bit field, stored as bytes.
    filter: Vec<u8>,
    /// Number of hash functions to use.
    hash_funcs: u32,
    /// Random tweak for the hash seed.
    tweak: u32,
    /// Filter update flags.
    flags: UpdateType,
    /// Number of elements the filter was designed for.
    elem_count: usize,
}

impl BloomFilter {
    /// Creates a new Bloom filter sized for `elem_count` elements with the
    /// given false-positive rate.
    ///
    /// The filter size and hash function count are calculated per BIP37.
    pub fn new(fp_rate: f64, elem_count: usize, tweak: u32, flags: UpdateType) -> Self {
        let n = elem_count as f64;

        // Filter size in bytes, capped at BLOOM_MAX_FILTER_LENGTH.
        let size = if elem_count == 0 {
            1
        } else {
            let raw = (-1.0 / LN2_SQUARED * n * fp_rate.ln() / 8.0) as usize;
            raw.clamp(1, BLOOM_MAX_FILTER_LENGTH)
        };

        // Number of hash functions, capped at BLOOM_MAX_HASH_FUNCS.
        let hash_funcs = if elem_count == 0 {
            0
        } else {
            let raw = (size as f64 * 8.0 / n * LN2) as u32;
            raw.min(BLOOM_MAX_HASH_FUNCS)
        };

        BloomFilter {
            filter: vec![0u8; size],
            hash_funcs,
            tweak,
            flags,
            elem_count,
        }
    }

    /// Computes the MurmurHash3 32-bit hash of `data` with the given `seed`.
    ///
    /// This is the standard MurmurHash3_x86_32 algorithm with:
    ///   c1 = 0xCC9E2D51
    ///   c2 = 0x1B873593
    fn murmur3_32(data: &[u8], seed: u32) -> u32 {
        const C1: u32 = 0xCC9E2D51;
        const C2: u32 = 0x1B873593;

        let len = data.len();
        let nblocks = len / 4;
        let mut h1 = seed;

        // Process 4-byte blocks.
        for i in 0..nblocks {
            let offset = i * 4;
            let k = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);

            let mut k1 = k;
            k1 = k1.wrapping_mul(C1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(C2);

            h1 ^= k1;
            h1 = h1.rotate_left(13);
            h1 = h1.wrapping_mul(5).wrapping_add(0xE6546B64);
        }

        // Handle tail bytes.
        let tail_offset = nblocks * 4;
        let tail = &data[tail_offset..];
        let mut k1: u32 = 0;

        match tail.len() {
            3 => {
                k1 ^= (tail[2] as u32) << 16;
                k1 ^= (tail[1] as u32) << 8;
                k1 ^= tail[0] as u32;
                k1 = k1.wrapping_mul(C1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(C2);
                h1 ^= k1;
            },
            2 => {
                k1 ^= (tail[1] as u32) << 8;
                k1 ^= tail[0] as u32;
                k1 = k1.wrapping_mul(C1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(C2);
                h1 ^= k1;
            },
            1 => {
                k1 ^= tail[0] as u32;
                k1 = k1.wrapping_mul(C1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(C2);
                h1 ^= k1;
            },
            _ => {},
        }

        // Finalization mix.
        h1 ^= len as u32;
        h1 ^= h1 >> 16;
        h1 = h1.wrapping_mul(0x85EBCA6B);
        h1 ^= h1 >> 13;
        h1 = h1.wrapping_mul(0xC2B2AE35);
        h1 ^= h1 >> 16;

        h1
    }

    /// Computes the BIP37 hash for the given data and hash function index `i`.
    ///
    /// `seed = i * MURMUR3_SEED_MULTIPLIER + tweak`
    /// Returns `murmur3_32(data, seed) % (filter_len * 8)`.
    fn hash(&self, data: &[u8], i: u32) -> u32 {
        let seed = i
            .wrapping_mul(MURMUR3_SEED_MULTIPLIER)
            .wrapping_add(self.tweak);
        let bits = (self.filter.len() as u32) * 8;
        Self::murmur3_32(data, seed) % bits
    }

    /// Inserts an element into the filter.
    pub fn insert(&mut self, data: &[u8]) {
        for i in 0..self.hash_funcs {
            let bit_index = self.hash(data, i);
            self.filter[(bit_index >> 3) as usize] |= 1 << (bit_index & 7);
        }
    }

    /// Tests whether an element is (probably) in the filter.
    ///
    /// Returns `true` if the element might be present, `false` if it is
    /// definitely not present.
    pub fn contains(&self, data: &[u8]) -> bool {
        for i in 0..self.hash_funcs {
            let bit_index = self.hash(data, i);
            if self.filter[(bit_index >> 3) as usize] & (1 << (bit_index & 7)) == 0 {
                return false;
            }
        }
        true
    }

    /// Serializes the filter to Bitcoin wire format.
    ///
    /// Wire format:
    ///   - compact_int: filter length
    ///   - [u8]: filter bytes
    ///   - u32 LE: hash_funcs
    ///   - u32 LE: tweak
    ///   - u8: flags
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        CompactInteger::from(self.filter.len()).encode(&mut stream);
        stream.append_raw_slice(&self.filter);
        stream.append(&self.hash_funcs);
        stream.append(&self.tweak);
        stream.append(&(self.flags as u8));
        stream.out()
    }

    /// Parses a Bloom filter from Bitcoin wire format.
    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);

        let filter_len = reader.read_compact_int()? as usize;
        if filter_len > BLOOM_MAX_FILTER_LENGTH {
            return Err(SpvError::InvalidData(format!(
                "bloom filter length {} exceeds maximum {}",
                filter_len, BLOOM_MAX_FILTER_LENGTH
            )));
        }

        let filter = reader.read_bytes(filter_len)?;
        let hash_funcs = reader.read_u32_le()?;
        if hash_funcs > BLOOM_MAX_HASH_FUNCS {
            return Err(SpvError::InvalidData(format!(
                "bloom filter hash_funcs {} exceeds maximum {}",
                hash_funcs, BLOOM_MAX_HASH_FUNCS
            )));
        }

        let tweak = reader.read_u32_le()?;
        let flags_byte = reader.read_u8()?;
        let flags = UpdateType::try_from(flags_byte)?;

        Ok(BloomFilter {
            filter,
            hash_funcs,
            tweak,
            flags,
            // We don't know the original elem_count from wire data; store 0.
            elem_count: 0,
        })
    }

    /// Returns the filter bytes.
    pub fn filter(&self) -> &[u8] {
        &self.filter
    }

    /// Returns the number of hash functions.
    pub fn hash_funcs(&self) -> u32 {
        self.hash_funcs
    }

    /// Returns the tweak value.
    pub fn tweak(&self) -> u32 {
        self.tweak
    }

    /// Returns the update flags.
    pub fn flags(&self) -> UpdateType {
        self.flags
    }

    /// Returns the element count the filter was designed for.
    pub fn elem_count(&self) -> usize {
        self.elem_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_insert_and_contains() {
        let mut filter = BloomFilter::new(0.01, 10, 0, UpdateType::None);

        let element = b"hello world";
        filter.insert(element);

        assert!(filter.contains(element));
        assert!(!filter.contains(b"goodbye world"));
    }

    #[test]
    fn test_bloom_filter_serialization_roundtrip() {
        let mut filter = BloomFilter::new(0.01, 10, 2147483649, UpdateType::P2PubkeyOnly);

        filter.insert(b"data1");
        filter.insert(b"data2");
        filter.insert(b"data3");

        let serialized = filter.serialize();
        let parsed = BloomFilter::parse(&serialized).expect("parse should succeed");

        assert!(parsed.contains(b"data1"));
        assert!(parsed.contains(b"data2"));
        assert!(parsed.contains(b"data3"));
        assert!(!parsed.contains(b"data4"));
        assert_eq!(parsed.hash_funcs(), filter.hash_funcs());
        assert_eq!(parsed.tweak(), filter.tweak());
        assert_eq!(parsed.flags(), filter.flags());
        assert_eq!(parsed.filter(), filter.filter());
    }

    #[test]
    fn test_bloom_filter_false_positive_rate() {
        let n = 100;
        let fp_rate = 0.01;
        let mut filter = BloomFilter::new(fp_rate, n, 42, UpdateType::None);

        // Insert n elements.
        for i in 0..n {
            let elem = format!("element-{}", i);
            filter.insert(elem.as_bytes());
        }

        // All inserted elements must be found.
        for i in 0..n {
            let elem = format!("element-{}", i);
            assert!(
                filter.contains(elem.as_bytes()),
                "inserted element {} not found",
                i
            );
        }

        // Count false positives among non-members.
        let test_count = 10_000;
        let mut false_positives = 0;
        for i in 0..test_count {
            let non_member = format!("nonmember-{}", i);
            if filter.contains(non_member.as_bytes()) {
                false_positives += 1;
            }
        }

        let observed_rate = false_positives as f64 / test_count as f64;
        assert!(
            observed_rate < 0.03,
            "false positive rate {} exceeds 3%",
            observed_rate
        );
    }

    #[test]
    fn test_bloom_filter_max_size() {
        // A huge elem_count should cap the filter size at BLOOM_MAX_FILTER_LENGTH.
        let filter = BloomFilter::new(0.00001, 100_000_000, 0, UpdateType::None);
        assert!(
            filter.filter().len() <= BLOOM_MAX_FILTER_LENGTH,
            "filter length {} exceeds max {}",
            filter.filter().len(),
            BLOOM_MAX_FILTER_LENGTH
        );
    }

    #[test]
    fn test_bloom_filter_empty() {
        let filter = BloomFilter::new(0.01, 10, 0, UpdateType::None);

        // An empty filter should not contain anything.
        assert!(!filter.contains(b"anything"));
        assert!(!filter.contains(b""));
        assert!(!filter.contains(b"test data"));
    }
}
