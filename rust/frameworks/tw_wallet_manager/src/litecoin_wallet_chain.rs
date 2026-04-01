use crate::wallet_chain::WalletChain;

/// Litecoin mainnet wallet policy.
pub struct LitecoinMainnetWallet;

impl WalletChain for LitecoinMainnetWallet {
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
        2
    }
}

/// Litecoin testnet wallet policy.
pub struct LitecoinTestnetWallet;

impl WalletChain for LitecoinTestnetWallet {
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
    fn test_mainnet_coin_type() {
        assert_eq!(LitecoinMainnetWallet::coin_type(), 2);
    }

    #[test]
    fn test_gap_limits() {
        assert_eq!(LitecoinMainnetWallet::external_gap_limit(), 20);
        assert_eq!(LitecoinMainnetWallet::internal_gap_limit(), 20);
    }

    #[test]
    fn test_dust_threshold() {
        let wallet = LitecoinMainnetWallet;
        // At fee_rate=1: dynamic=0, dust=546
        assert_eq!(wallet.dust_threshold(1), 546);
        // At fee_rate=10000: 10000*3*182/1000 = 5460
        assert_eq!(wallet.dust_threshold(10000), 5460);
    }
}
