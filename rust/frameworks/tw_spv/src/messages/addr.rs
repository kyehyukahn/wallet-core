// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::decode::Reader;
use crate::error::SpvResult;
use tw_utxo::encode::compact_integer::CompactInteger;
use tw_utxo::encode::stream::Stream;
use tw_utxo::encode::Encodable;

/// A network address entry in an `addr` message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAddress {
    pub timestamp: u32,
    pub services: u64,
    pub address: [u8; 16],
    pub port: u16,
}

/// An `addr` message containing a list of known network addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrMessage {
    pub addresses: Vec<NetworkAddress>,
}

impl AddrMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut stream = Stream::new();
        CompactInteger::from(self.addresses.len()).encode(&mut stream);
        for addr in &self.addresses {
            stream.append(&addr.timestamp);
            stream.append(&addr.services);
            stream.append_raw_slice(&addr.address);
            stream.append_raw_slice(&addr.port.to_be_bytes());
        }
        stream.out()
    }

    pub fn parse(data: &[u8]) -> SpvResult<Self> {
        let mut reader = Reader::new(data);
        let count = reader.read_compact_int()? as usize;
        let mut addresses = Vec::with_capacity(count);
        for _ in 0..count {
            let timestamp = reader.read_u32_le()?;
            let services = reader.read_u64_le()?;
            let addr_bytes = reader.read_bytes(16)?;
            let mut address = [0u8; 16];
            address.copy_from_slice(&addr_bytes);
            let port_bytes = reader.read_bytes(2)?;
            let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);
            addresses.push(NetworkAddress {
                timestamp,
                services,
                address,
                port,
            });
        }
        Ok(AddrMessage { addresses })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addr_roundtrip() {
        let mut address = [0u8; 16];
        address[10] = 0xFF;
        address[11] = 0xFF;
        address[12] = 192;
        address[13] = 168;
        address[14] = 1;
        address[15] = 1;

        let msg = AddrMessage {
            addresses: vec![NetworkAddress {
                timestamp: 1700000000,
                services: 1,
                address,
                port: 8333,
            }],
        };
        let serialized = msg.serialize();
        let parsed = AddrMessage::parse(&serialized).unwrap();
        assert_eq!(parsed, msg);
    }
}
