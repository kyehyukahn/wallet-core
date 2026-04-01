/// Trait defining chain-specific wallet policies.
pub trait WalletChain {
    /// Number of consecutive unused external (receiving) addresses to scan.
    fn external_gap_limit() -> u32;

    /// Number of consecutive unused internal (change) addresses to scan.
    fn internal_gap_limit() -> u32;

    /// Minimum output value considered non-dust at the given fee rate (sat/vB).
    fn dust_threshold(&self, fee_rate: u64) -> u64;

    /// Maximum expected reorganization depth in blocks.
    fn max_reorg_depth() -> u32;

    /// BIP-44 coin type for this chain.
    fn coin_type() -> u32;
}
