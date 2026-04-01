// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::transaction::transaction_parts::Amount;

pub mod fee_estimator;

/// Standard fee policy.
pub enum FeePolicy {
    // The amount of satoshis per vbyte ("satVb"), used for fee calculation.
    // Can be satoshis per byte ("satB") **ONLY** when transaction does not contain segwit UTXOs.
    FeePerVb(Amount),
    /// Fee rate in satoshis per kilobyte.
    FeePerKb(Amount),
}

impl FeePolicy {
    /// Returns the fee rate in satoshis per virtual byte.
    /// For `FeePerKb`, converts by dividing by 1000 and rounding up.
    pub fn sat_per_vbyte(&self) -> Amount {
        match self {
            FeePolicy::FeePerVb(rate) => *rate,
            FeePolicy::FeePerKb(rate) => (*rate + 999) / 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_per_vb() {
        let policy = FeePolicy::FeePerVb(10);
        assert_eq!(policy.sat_per_vbyte(), 10);
    }

    #[test]
    fn test_fee_per_kb_exact() {
        let policy = FeePolicy::FeePerKb(10000);
        assert_eq!(policy.sat_per_vbyte(), 10);
    }

    #[test]
    fn test_fee_per_kb_rounds_up() {
        let policy = FeePolicy::FeePerKb(1001);
        assert_eq!(policy.sat_per_vbyte(), 2);
    }

    #[test]
    fn test_fee_per_kb_minimum() {
        let policy = FeePolicy::FeePerKb(1);
        assert_eq!(policy.sat_per_vbyte(), 1);
    }
}
