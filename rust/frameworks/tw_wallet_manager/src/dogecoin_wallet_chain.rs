use crate::wallet_chain::WalletChain;

/// 1 DOGE in satoshis.
const ONE_DOGE: u64 = 100_000_000;

/// Dogecoin mainnet wallet policy.
pub struct DogecoinMainnetWallet;

impl WalletChain for DogecoinMainnetWallet {
    fn external_gap_limit() -> u32 {
        20
    }

    fn internal_gap_limit() -> u32 {
        20
    }

    fn dust_threshold(&self, fee_rate: u64) -> u64 {
        let dynamic = fee_rate * 3 * 182 / 1000;
        std::cmp::max(dynamic, ONE_DOGE)
    }

    fn max_reorg_depth() -> u32 {
        6
    }

    fn coin_type() -> u32 {
        3
    }
}

/// Dogecoin testnet wallet policy.
pub struct DogecoinTestnetWallet;

impl WalletChain for DogecoinTestnetWallet {
    fn external_gap_limit() -> u32 {
        20
    }

    fn internal_gap_limit() -> u32 {
        20
    }

    fn dust_threshold(&self, fee_rate: u64) -> u64 {
        let dynamic = fee_rate * 3 * 182 / 1000;
        std::cmp::max(dynamic, ONE_DOGE)
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
    fn test_mainnet_coin_type() {
        assert_eq!(DogecoinMainnetWallet::coin_type(), 3);
    }

    #[test]
    fn test_dust_minimum_is_one_doge() {
        let wallet = DogecoinMainnetWallet;
        // At low fee rate, dust should be 1 DOGE minimum
        assert_eq!(wallet.dust_threshold(1), 100_000_000);
    }

    #[test]
    fn test_dust_at_high_fee_still_one_doge() {
        let wallet = DogecoinMainnetWallet;
        // At fee_rate=10000: 10000*3*182/1000 = 5460, still below 1 DOGE
        assert_eq!(wallet.dust_threshold(10000), 100_000_000);
        // Even at fee_rate=100000: 100000*3*182/1000 = 54600, still below 1 DOGE
        assert_eq!(wallet.dust_threshold(100_000), 100_000_000);
    }

    #[test]
    fn test_gap_limits() {
        assert_eq!(DogecoinMainnetWallet::external_gap_limit(), 20);
        assert_eq!(DogecoinMainnetWallet::internal_gap_limit(), 20);
    }
}
