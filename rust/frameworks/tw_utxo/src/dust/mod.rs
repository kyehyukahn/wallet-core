// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::transaction::transaction_parts::Amount;

pub mod dust_filter;

/// Estimated size of a typical transaction output (P2PKH).
const TX_OUTPUT_SIZE: Amount = 34;
/// Estimated size of a typical transaction input (compressed pubkey).
const TX_INPUT_SIZE: Amount = 148;

/// Transaction dust amount calculator.
#[derive(Clone, Copy)]
pub enum DustPolicy {
    FixedAmount(Amount),
    /// Dynamic dust based on fee rate.
    /// threshold = max(fee_per_kb * 3 * (34 + 148) / 1000, min_amount)
    DynamicDust {
        fee_per_kb: Amount,
        min_amount: Amount,
    },
}

impl DustPolicy {
    pub fn dust_threshold(&self) -> Amount {
        match self {
            DustPolicy::FixedAmount(amount) => *amount,
            DustPolicy::DynamicDust {
                fee_per_kb,
                min_amount,
            } => {
                let calculated = fee_per_kb * 3 * (TX_OUTPUT_SIZE + TX_INPUT_SIZE) / 1000;
                std::cmp::max(calculated, *min_amount)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_dust_threshold() {
        let policy = DustPolicy::FixedAmount(546);
        assert_eq!(policy.dust_threshold(), 546);
    }

    #[test]
    fn test_dynamic_dust_standard_fee() {
        let policy = DustPolicy::DynamicDust {
            fee_per_kb: 10000,
            min_amount: 546,
        };
        assert_eq!(policy.dust_threshold(), 5460);
    }

    #[test]
    fn test_dynamic_dust_low_fee_uses_minimum() {
        let policy = DustPolicy::DynamicDust {
            fee_per_kb: 1,
            min_amount: 546,
        };
        assert_eq!(policy.dust_threshold(), 546);
    }

    #[test]
    fn test_dynamic_dust_high_fee() {
        let policy = DustPolicy::DynamicDust {
            fee_per_kb: 50000,
            min_amount: 546,
        };
        assert_eq!(policy.dust_threshold(), 27300);
    }
}
