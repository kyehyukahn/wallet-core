use crate::error::{WalletError, WalletResult};
use crate::wallet_chain::WalletChain;
use bitcoin::bip32::{ChildNumber, ExtendedPubKey};
use bitcoin::secp256k1::Secp256k1;
use std::collections::HashSet;

/// Manages BIP32-derived addresses for a wallet, maintaining gap limit rules
/// for both external (receiving) and internal (change) address chains.
pub struct AddressManager {
    xpub: ExtendedPubKey,
    secp: Secp256k1<bitcoin::secp256k1::All>,
    external_cursor: u32,
    internal_cursor: u32,
    external_addresses: Vec<String>,
    internal_addresses: Vec<String>,
    used_addresses: HashSet<String>,
    all_addresses: HashSet<String>,
    external_gap_limit: u32,
    internal_gap_limit: u32,
}

impl AddressManager {
    /// Create a new AddressManager and pre-fill gap limit addresses for both chains.
    pub fn new<C: WalletChain>(xpub: ExtendedPubKey, _chain: &C) -> WalletResult<Self> {
        let mut mgr = Self {
            xpub,
            secp: Secp256k1::new(),
            external_cursor: 0,
            internal_cursor: 0,
            external_addresses: Vec::new(),
            internal_addresses: Vec::new(),
            used_addresses: HashSet::new(),
            all_addresses: HashSet::new(),
            external_gap_limit: C::external_gap_limit(),
            internal_gap_limit: C::internal_gap_limit(),
        };
        mgr.ensure_gap_limit()?;
        Ok(mgr)
    }

    /// Derive a HASH160 address (hex-encoded) for the given chain and index.
    ///
    /// Performs BIP32 derivation: xpub / chain / index, then computes
    /// HASH160 (SHA256 followed by RIPEMD160) of the compressed public key.
    pub fn derive_address(&self, chain: u32, index: u32) -> WalletResult<String> {
        let chain_child = ChildNumber::from_normal_idx(chain)
            .map_err(|e| WalletError::AddressDerivation(format!("invalid chain index: {e}")))?;
        let index_child = ChildNumber::from_normal_idx(index)
            .map_err(|e| WalletError::AddressDerivation(format!("invalid index: {e}")))?;

        let chain_xpub = self
            .xpub
            .ckd_pub(&self.secp, chain_child)
            .map_err(|e| WalletError::AddressDerivation(format!("chain derivation failed: {e}")))?;
        let child_xpub = chain_xpub
            .ckd_pub(&self.secp, index_child)
            .map_err(|e| WalletError::AddressDerivation(format!("index derivation failed: {e}")))?;

        let pubkey_bytes = child_xpub.public_key.serialize();
        let hash160 = tw_hash::ripemd::ripemd_160(&tw_hash::sha2::sha256(&pubkey_bytes));
        Ok(tw_encoding::hex::encode(&hash160, false))
    }

    /// Ensure the gap limit is satisfied for both external and internal chains.
    ///
    /// Returns the number of new addresses generated for (external, internal).
    pub fn ensure_gap_limit(&mut self) -> WalletResult<(u32, u32)> {
        let ext_new = self.fill_chain(0)?;
        let int_new = self.fill_chain(1)?;
        Ok((ext_new, int_new))
    }

    /// Fill a single chain (0 = external, 1 = internal) until the gap limit is met.
    fn fill_chain(&mut self, chain: u32) -> WalletResult<u32> {
        let gap_limit = if chain == 0 {
            self.external_gap_limit
        } else {
            self.internal_gap_limit
        };

        let mut generated = 0u32;

        loop {
            let (addresses, cursor) = if chain == 0 {
                (&self.external_addresses, &self.external_cursor)
            } else {
                (&self.internal_addresses, &self.internal_cursor)
            };

            // Count consecutive unused addresses at the tail.
            let unused_tail = addresses
                .iter()
                .rev()
                .take_while(|addr| !self.used_addresses.contains(*addr))
                .count() as u32;

            if unused_tail >= gap_limit {
                break;
            }

            let current_cursor = *cursor;
            let addr = self.derive_address(chain, current_cursor)?;
            self.all_addresses.insert(addr.clone());

            if chain == 0 {
                self.external_addresses.push(addr);
                self.external_cursor += 1;
            } else {
                self.internal_addresses.push(addr);
                self.internal_cursor += 1;
            }
            generated += 1;
        }

        Ok(generated)
    }

    /// Mark an address as used. Returns true if the address is known to this manager.
    pub fn mark_address_used(&mut self, address: &str) -> bool {
        if self.all_addresses.contains(address) {
            self.used_addresses.insert(address.to_string());
            true
        } else {
            false
        }
    }

    /// Check whether the given address is managed by this address manager.
    pub fn contains_address(&self, address: &str) -> bool {
        self.all_addresses.contains(address)
    }

    /// Check whether the given address belongs to the internal (change) chain.
    pub fn is_change_address(&self, address: &str) -> bool {
        self.internal_addresses.iter().any(|a| a == address)
    }

    /// Return the first unused external (receiving) address.
    pub fn receive_address(&self) -> Option<&str> {
        self.external_addresses
            .iter()
            .find(|addr| !self.used_addresses.contains(addr.as_str()))
            .map(|s| s.as_str())
    }

    /// Return the first unused internal (change) address.
    pub fn change_address(&self) -> Option<&str> {
        self.internal_addresses
            .iter()
            .find(|addr| !self.used_addresses.contains(addr.as_str()))
            .map(|s| s.as_str())
    }

    /// Return all known addresses.
    pub fn all_addresses(&self) -> &HashSet<String> {
        &self.all_addresses
    }

    /// Return all used addresses.
    pub fn used_addresses(&self) -> &HashSet<String> {
        &self.used_addresses
    }

    /// Return the current cursors for persistence: (external_cursor, internal_cursor).
    pub fn cursors(&self) -> (u32, u32) {
        (self.external_cursor, self.internal_cursor)
    }

    /// Restore an AddressManager from persisted state.
    ///
    /// Re-derives all addresses up to the given cursors, marks the provided
    /// addresses as used, then ensures the gap limit is satisfied.
    pub fn restore(
        xpub: ExtendedPubKey,
        ext_cursor: u32,
        int_cursor: u32,
        used: HashSet<String>,
        external_gap_limit: u32,
        internal_gap_limit: u32,
    ) -> WalletResult<Self> {
        let secp = Secp256k1::new();
        let mut external_addresses = Vec::with_capacity(ext_cursor as usize);
        let mut internal_addresses = Vec::with_capacity(int_cursor as usize);
        let mut all_addresses = HashSet::new();

        let mut mgr_tmp = Self {
            xpub,
            secp,
            external_cursor: 0,
            internal_cursor: 0,
            external_addresses: Vec::new(),
            internal_addresses: Vec::new(),
            used_addresses: HashSet::new(),
            all_addresses: HashSet::new(),
            external_gap_limit,
            internal_gap_limit,
        };

        // Re-derive external addresses.
        for i in 0..ext_cursor {
            let addr = mgr_tmp.derive_address(0, i)?;
            all_addresses.insert(addr.clone());
            external_addresses.push(addr);
        }

        // Re-derive internal addresses.
        for i in 0..int_cursor {
            let addr = mgr_tmp.derive_address(1, i)?;
            all_addresses.insert(addr.clone());
            internal_addresses.push(addr);
        }

        mgr_tmp.external_addresses = external_addresses;
        mgr_tmp.internal_addresses = internal_addresses;
        mgr_tmp.external_cursor = ext_cursor;
        mgr_tmp.internal_cursor = int_cursor;
        mgr_tmp.all_addresses = all_addresses;
        mgr_tmp.used_addresses = used
            .into_iter()
            .filter(|a| mgr_tmp.all_addresses.contains(a))
            .collect();

        mgr_tmp.ensure_gap_limit()?;
        Ok(mgr_tmp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin_wallet_chain::BitcoinMainnetWallet;
    use std::str::FromStr;

    fn test_xpub() -> ExtendedPubKey {
        ExtendedPubKey::from_str(
            "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8"
        ).unwrap()
    }

    #[test]
    fn test_address_manager_initial_gap_fill() {
        let mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        assert_eq!(mgr.external_addresses.len(), 20);
        assert_eq!(mgr.internal_addresses.len(), 20);
        assert_eq!(mgr.cursors(), (20, 20));
    }

    #[test]
    fn test_receive_and_change_addresses() {
        let mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        let recv = mgr.receive_address().unwrap();
        let change = mgr.change_address().unwrap();
        assert!(!recv.is_empty());
        assert!(!change.is_empty());
        assert_ne!(recv, change);
    }

    #[test]
    fn test_mark_used_triggers_gap_fill() {
        let mut mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        let first_addr = mgr.external_addresses[0].clone();
        assert!(mgr.mark_address_used(&first_addr));
        let (ext_new, _) = mgr.ensure_gap_limit().unwrap();
        assert_eq!(ext_new, 1);
        assert_eq!(mgr.external_addresses.len(), 21);
    }

    #[test]
    fn test_contains_address() {
        let mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        let known = &mgr.external_addresses[0];
        assert!(mgr.contains_address(known));
        assert!(!mgr.contains_address("deadbeef00000000000000000000000000000000"));
    }

    #[test]
    fn test_is_change_address() {
        let mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        let ext_addr = &mgr.external_addresses[0];
        let int_addr = &mgr.internal_addresses[0];
        assert!(!mgr.is_change_address(ext_addr));
        assert!(mgr.is_change_address(int_addr));
    }

    #[test]
    fn test_restore_from_persistence() {
        let mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        let (ext_cur, int_cur) = mgr.cursors();
        let used = mgr.used_addresses.clone();

        let restored =
            AddressManager::restore(test_xpub(), ext_cur, int_cur, used, 20, 20).unwrap();

        assert_eq!(restored.external_addresses, mgr.external_addresses);
        assert_eq!(restored.internal_addresses, mgr.internal_addresses);
        assert_eq!(restored.cursors(), mgr.cursors());
    }

    #[test]
    fn test_mark_unknown_address_returns_false() {
        let mut mgr = AddressManager::new(test_xpub(), &BitcoinMainnetWallet).unwrap();
        assert!(!mgr.mark_address_used("not_a_real_address"));
    }
}
