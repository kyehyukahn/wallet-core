// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::messages::inv::InvMessage;

/// A `getdata` message has the same format as an `inv` message.
pub type GetDataMessage = InvMessage;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::inv::{InvType, InvVector};
    use tw_hash::H256;

    #[test]
    fn test_getdata_roundtrip_filtered_witness_block() {
        let msg = GetDataMessage {
            inventory: vec![InvVector {
                inv_type: InvType::FilteredWitnessBlock,
                hash: H256::from([0xCC; 32]),
            }],
        };
        let serialized = msg.serialize();
        let parsed = GetDataMessage::parse(&serialized).unwrap();
        assert_eq!(parsed, msg);
    }
}
