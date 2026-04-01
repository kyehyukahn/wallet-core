// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::bloom_filter::BloomFilter;

/// A `filterload` message wrapping a BIP37 Bloom filter.
pub struct FilterLoadMessage;

impl FilterLoadMessage {
    /// Serializes a Bloom filter into the `filterload` message payload.
    pub fn from_bloom_filter(filter: &BloomFilter) -> Vec<u8> {
        filter.serialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bloom_filter::UpdateType;

    #[test]
    fn test_filterload_roundtrip() {
        let mut filter = BloomFilter::new(0.01, 10, 42, UpdateType::All);
        filter.insert(b"hello");
        filter.insert(b"world");

        let payload = FilterLoadMessage::from_bloom_filter(&filter);
        let parsed = BloomFilter::parse(&payload).unwrap();

        assert!(parsed.contains(b"hello"));
        assert!(parsed.contains(b"world"));
        assert!(!parsed.contains(b"missing"));
        assert_eq!(parsed.hash_funcs(), filter.hash_funcs());
        assert_eq!(parsed.tweak(), filter.tweak());
        assert_eq!(parsed.flags(), filter.flags());
    }
}
