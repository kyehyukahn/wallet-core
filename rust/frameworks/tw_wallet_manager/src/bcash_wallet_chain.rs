use crate::wallet_chain::WalletChain;

/// Bitcoin Cash mainnet wallet policy.
pub struct BitcoinCashMainnetWallet;

impl WalletChain for BitcoinCashMainnetWallet {
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
        10
    }

    fn coin_type() -> u32 {
        145
    }
}

/// Bitcoin Cash testnet wallet policy.
pub struct BitcoinCashTestnetWallet;

impl WalletChain for BitcoinCashTestnetWallet {
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
        10
    }

    fn coin_type() -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mainnet_coin_type() {
        assert_eq!(BitcoinCashMainnetWallet::coin_type(), 145);
    }

    #[test]
    fn test_testnet_coin_type() {
        assert_eq!(BitcoinCashTestnetWallet::coin_type(), 1);
    }

    #[test]
    fn test_gap_limits() {
        assert_eq!(BitcoinCashMainnetWallet::external_gap_limit(), 20);
        assert_eq!(BitcoinCashMainnetWallet::internal_gap_limit(), 20);
    }

    #[test]
    fn test_dust_at_10000() {
        let wallet = BitcoinCashMainnetWallet;
        // At fee_rate=10000: 10000*3*182/1000 = 5460
        assert_eq!(wallet.dust_threshold(10000), 5460);
    }

    #[test]
    fn test_reorg_depth() {
        assert_eq!(BitcoinCashMainnetWallet::max_reorg_depth(), 10);
    }
}
