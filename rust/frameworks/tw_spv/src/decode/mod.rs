// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

mod reader;

pub use reader::Reader;

use crate::error::SpvResult;

/// Trait for types that can be decoded from the Bitcoin wire format.
pub trait Decodable: Sized {
    fn decode(reader: &mut Reader<'_>) -> SpvResult<Self>;
}
