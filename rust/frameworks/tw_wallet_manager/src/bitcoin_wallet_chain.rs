use crate::wallet_chain::WalletChain;

/// Bitcoin mainnet wallet policy.
pub struct BitcoinMainnetWallet;

impl WalletChain for BitcoinMainnetWallet {
    fn external_gap_limit() -> u32 {
        20
    }

    fn internal_gap_limit() -> u32 {
        20
    }

    fn dust_threshold(&self, fee_rate: u64) -> u64 {
        let dynamic = fee_rate * 3 * 182 / 1000;
        std::cmp::max(dynamic, 546)
    }

    fn max_reorg_depth() -> u32 {
        6
    }

    fn coin_type() -> u32 {
        0
    }
}

/// Bitcoin testnet wallet policy.
pub struct BitcoinTestnetWallet;

impl WalletChain for BitcoinTestnetWallet {
    fn external_gap_limit() -> u32 {
        20
    }

    fn internal_gap_limit() -> u32 {
        20
    }

    fn dust_threshold(&self, fee_rate: u64) -> u64 {
        let dynamic = fee_rate * 3 * 182 / 1000;
        std::cmp::max(dynamic, 546)
    }

    fn max_reorg_depth() -> u32 {
        6
    }

    fn coin_type() -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_limits() {
        assert_eq!(BitcoinMainnetWallet::external_gap_limit(), 20);
        assert_eq!(BitcoinMainnetWallet::internal_gap_limit(), 20);
    }

    #[test]
    fn test_testnet_coin_type() {
        assert_eq!(BitcoinTestnetWallet::coin_type(), 1);
    }

    #[test]
    fn test_dust_at_default_rate() {
        let wallet = BitcoinMainnetWallet;
        // At fee_rate=1: 1*3*182/1000 = 0, so dust = max(0, 546) = 546
        assert_eq!(wallet.dust_threshold(1), 546);
    }

    #[test]
    fn test_dust_minimum() {
        let wallet = BitcoinMainnetWallet;
        // At fee_rate=10: 10*3*182/1000 = 5460/1000 = 5 (integer div), so dust = max(5, 546) = 546
        assert_eq!(wallet.dust_threshold(10), 546);
        // At fee_rate=1000: 1000*3*182/1000 = 546, so dust = max(546, 546) = 546
        assert_eq!(wallet.dust_threshold(1000), 546);
        // At fee_rate=2000: 2000*3*182/1000 = 1092, so dust = max(1092, 546) = 1092
        assert_eq!(wallet.dust_threshold(2000), 1092);
    }
}
