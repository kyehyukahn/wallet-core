// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::{SpvError, SpvResult};
use tw_hash::H256;
use tw_utxo::encode::compact_integer::CompactInteger;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

/// Witness flag for segregated witness inventory types.
pub const WITNESS_FLAG: u32 = 0x40000000;

/// Inventory vector type identifiers as defined in the Bitcoin P2P protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvType {
    Tx,
    Block,
    FilteredBlock,
    WitnessTx,
    WitnessBlock,
    FilteredWitnessBlock,
}

impl InvType {
    pub fn from_u32(v: u32) -> SpvResult<Self> {
        match v {
            1 => Ok(InvType::Tx),
            2 => Ok(InvType::Block),
            3 => Ok(InvType::FilteredBlock),
            v if v == 1 | WITNESS_FLAG => Ok(InvType::WitnessTx),
            v if v == 2 | WITNESS_FLAG => Ok(InvType::WitnessBlock),
            v if v == 3 | WITNESS_FLAG => Ok(InvType::FilteredWitnessBlock),
            _ => Err(SpvError::InvalidData(format!(
                "unknown inventory type: {}",
                v
            ))),
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            InvType::Tx => 1,
            InvType::Block => 2,
            InvType::FilteredBlock => 3,
            InvType::WitnessTx => 1 | WITNESS_FLAG,
            InvType::WitnessBlock => 2 | WITNESS_FLAG,
            InvType::FilteredWitnessBlock => 3 | WITNESS_FLAG,
        }
    }
}

/// A single inventory vector: a type and a hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvVector {
    pub inv_type: InvType,
    pub hash: H256,
}

/// An `inv` message containing a list of inventory vectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvMessage {
    pub inventory: Vec<InvVector>,
}

impl InvMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        CompactInteger::from(self.inventory.len()).encode(&mut stream);
        for iv in &self.inventory {
            stream.append(&iv.inv_type.to_u32());
            stream.append(&iv.hash);
        }
        stream.out()
    }

    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let count = reader.read_compact_int()? as usize;
        let mut inventory = Vec::with_capacity(count);
        for _ in 0..count {
            let type_id = reader.read_u32_le()?;
            let inv_type = InvType::from_u32(type_id)?;
            let hash = reader.read_h256()?;
            inventory.push(InvVector { inv_type, hash });
        }
        Ok(InvMessage { inventory })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_message_roundtrip() {
        let msg = InvMessage {
            inventory: vec![
                InvVector {
                    inv_type: InvType::Tx,
                    hash: H256::from([0xAA; 32]),
                },
                InvVector {
                    inv_type: InvType::Block,
                    hash: H256::from([0xBB; 32]),
                },
            ],
        };
        let serialized = msg.serialize();
        let parsed = InvMessage::parse(&serialized).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn test_witness_type_conversion() {
        assert_eq!(InvType::WitnessTx.to_u32(), 1 | WITNESS_FLAG);
        assert_eq!(InvType::WitnessBlock.to_u32(), 2 | WITNESS_FLAG);
        assert_eq!(InvType::FilteredWitnessBlock.to_u32(), 3 | WITNESS_FLAG);

        assert_eq!(
            InvType::from_u32(1 | WITNESS_FLAG).unwrap(),
            InvType::WitnessTx
        );
        assert_eq!(
            InvType::from_u32(2 | WITNESS_FLAG).unwrap(),
            InvType::WitnessBlock
        );
        assert_eq!(
            InvType::from_u32(3 | WITNESS_FLAG).unwrap(),
            InvType::FilteredWitnessBlock
        );
    }
}
