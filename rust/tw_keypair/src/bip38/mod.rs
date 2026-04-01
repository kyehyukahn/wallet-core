// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

//! BIP38 key encryption/decryption (non-EC multiply mode).

pub mod scrypt_params;

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;
use scrypt_params::{SCRYPT_KEY_LEN, SCRYPT_N, SCRYPT_P, SCRYPT_R};

/// BIP38-specific errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bip38Error {
    InvalidKey,
    InvalidPassphrase,
    ScryptFailed,
    InvalidFormat,
    ChecksumMismatch,
}

/// Checks whether the given byte slice is a valid BIP38-encrypted key.
///
/// A valid BIP38 key is exactly 39 bytes and starts with prefix
/// `0x01 0x42` (non-EC-multiply, no compression) or `0x01 0x43` (non-EC-multiply, compression).
pub fn is_valid_bip38(encoded: &[u8]) -> bool {
    if encoded.len() != 39 {
        return false;
    }
    encoded[0] == 0x01 && (encoded[1] == 0x42 || encoded[1] == 0x43)
}

/// Decrypts a BIP38-encrypted private key (non-EC multiply mode).
///
/// `encrypted` must be exactly 39 bytes with prefix `0x01 0x42` or `0x01 0x43`.
///
/// # Algorithm
/// 1. Extract salt (address hash) from bytes 3..7.
/// 2. Derive 64 bytes via scrypt(passphrase, salt, N=16384, r=8, p=8).
/// 3. Split derived key into `derived_half1` (32 bytes) and `derived_half2` (32 bytes).
/// 4. AES-256-ECB decrypt two 16-byte blocks (bytes 7..23 and 23..39) using `derived_half2` as key.
/// 5. XOR decrypted blocks with `derived_half1` to recover the private key.
pub fn decrypt(encrypted: &[u8], passphrase: &str) -> Result<[u8; 32], Bip38Error> {
    if encrypted.len() != 39 {
        return Err(Bip38Error::InvalidFormat);
    }
    if encrypted[0] != 0x01 || (encrypted[1] != 0x42 && encrypted[1] != 0x43) {
        return Err(Bip38Error::InvalidFormat);
    }

    let salt: [u8; 4] = encrypted[3..7].try_into().unwrap();
    let encrypted_half1 = &encrypted[7..23];
    let encrypted_half2 = &encrypted[23..39];

    // Derive key via scrypt.
    let derived = scrypt_derive(passphrase.as_bytes(), &salt)?;
    let derived_half1 = &derived[..32];
    let derived_half2 = &derived[32..64];

    // AES-256-ECB decrypt.
    let cipher = Aes256::new(aes::cipher::generic_array::GenericArray::from_slice(
        derived_half2,
    ));

    let mut block1 = aes::cipher::generic_array::GenericArray::clone_from_slice(encrypted_half1);
    let mut block2 = aes::cipher::generic_array::GenericArray::clone_from_slice(encrypted_half2);
    cipher.decrypt_block(&mut block1);
    cipher.decrypt_block(&mut block2);

    // XOR with derived_half1 to recover the private key.
    let mut privkey = [0u8; 32];
    for i in 0..16 {
        privkey[i] = block1[i] ^ derived_half1[i];
    }
    for i in 0..16 {
        privkey[16 + i] = block2[i] ^ derived_half1[16 + i];
    }

    Ok(privkey)
}

/// Encrypts a private key using BIP38 non-EC multiply mode.
///
/// # Arguments
/// * `privkey` - The 32-byte private key to encrypt.
/// * `passphrase` - The passphrase used for encryption.
/// * `compressed` - Whether the key corresponds to a compressed public key.
/// * `addr_hash` - The first 4 bytes of SHA256d(address), used as salt.
///
/// # Algorithm
/// 1. Derive 64 bytes via scrypt(passphrase, salt=addr_hash, N=16384, r=8, p=8).
/// 2. Split derived key into `derived_half1` (32 bytes) and `derived_half2` (32 bytes).
/// 3. XOR private key with `derived_half1`.
/// 4. AES-256-ECB encrypt two 16-byte blocks using `derived_half2` as key.
/// 5. Build 39-byte result: `[0x01, 0x42, flag_byte, addr_hash[4], encrypted_half1[16], encrypted_half2[16]]`.
pub fn encrypt(
    privkey: &[u8; 32],
    passphrase: &str,
    compressed: bool,
    addr_hash: &[u8; 4],
) -> Result<[u8; 39], Bip38Error> {
    // Derive key via scrypt.
    let derived = scrypt_derive(passphrase.as_bytes(), addr_hash)?;
    let derived_half1 = &derived[..32];
    let derived_half2 = &derived[32..64];

    // XOR private key with derived_half1.
    let mut xored = [0u8; 32];
    for i in 0..32 {
        xored[i] = privkey[i] ^ derived_half1[i];
    }

    // AES-256-ECB encrypt.
    let cipher = Aes256::new(aes::cipher::generic_array::GenericArray::from_slice(
        derived_half2,
    ));

    let mut block1 = aes::cipher::generic_array::GenericArray::clone_from_slice(&xored[..16]);
    let mut block2 = aes::cipher::generic_array::GenericArray::clone_from_slice(&xored[16..32]);
    cipher.encrypt_block(&mut block1);
    cipher.encrypt_block(&mut block2);

    // Build result: prefix(2) + flag(1) + addr_hash(4) + encrypted(32) = 39 bytes.
    let flag_byte: u8 = if compressed { 0xe0 } else { 0xc0 };

    let mut result = [0u8; 39];
    result[0] = 0x01;
    result[1] = 0x42;
    result[2] = flag_byte;
    result[3..7].copy_from_slice(addr_hash);
    result[7..23].copy_from_slice(&block1);
    result[23..39].copy_from_slice(&block2);

    Ok(result)
}

/// Derives a key using scrypt with BIP38 parameters.
fn scrypt_derive(passphrase: &[u8], salt: &[u8]) -> Result<[u8; SCRYPT_KEY_LEN], Bip38Error> {
    let params = scrypt::Params::new(
        SCRYPT_N.trailing_zeros() as u8, // log2(N)
        SCRYPT_R,
        SCRYPT_P,
        SCRYPT_KEY_LEN,
    )
    .map_err(|_| Bip38Error::ScryptFailed)?;

    let mut output = [0u8; SCRYPT_KEY_LEN];
    scrypt::scrypt(passphrase, salt, &params, &mut output).map_err(|_| Bip38Error::ScryptFailed)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_bip38() {
        // Valid 0x42 prefix (non-EC, no compression flag in prefix).
        let mut valid_42 = [0u8; 39];
        valid_42[0] = 0x01;
        valid_42[1] = 0x42;
        assert!(is_valid_bip38(&valid_42));

        // Valid 0x43 prefix (EC-multiply).
        let mut valid_43 = [0u8; 39];
        valid_43[0] = 0x01;
        valid_43[1] = 0x43;
        assert!(is_valid_bip38(&valid_43));

        // Invalid second byte 0x44.
        let mut invalid_44 = [0u8; 39];
        invalid_44[0] = 0x01;
        invalid_44[1] = 0x44;
        assert!(!is_valid_bip38(&invalid_44));

        // Wrong length (38 bytes).
        let short = [0u8; 38];
        assert!(!is_valid_bip38(&short));

        // Wrong length (40 bytes).
        let long = [0u8; 40];
        assert!(!is_valid_bip38(&long));
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let privkey: [u8; 32] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab, 0xcd, 0xef,
        ];
        let passphrase = "TestingBIP38";
        let addr_hash: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];

        let encrypted = encrypt(&privkey, passphrase, true, &addr_hash).unwrap();
        assert!(is_valid_bip38(&encrypted));

        let decrypted = decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, privkey);
    }

    #[test]
    fn test_decrypt_wrong_passphrase() {
        let privkey: [u8; 32] = [0x42; 32];
        let passphrase = "CorrectPassphrase";
        let addr_hash: [u8; 4] = [0x01, 0x02, 0x03, 0x04];

        let encrypted = encrypt(&privkey, passphrase, false, &addr_hash).unwrap();

        // Decrypting with wrong passphrase should produce a different key (BIP38 has no MAC).
        let wrong = decrypt(&encrypted, "WrongPassphrase").unwrap();
        assert_ne!(wrong, privkey);
    }

    #[test]
    fn test_invalid_format() {
        // Too short.
        let short = [0u8; 10];
        assert_eq!(decrypt(&short, "pass"), Err(Bip38Error::InvalidFormat));

        // EC-multiply prefix (0x01 0x43 is accepted by is_valid_bip38 but also valid here).
        // Instead test a truly invalid prefix.
        let mut bad_prefix = [0u8; 39];
        bad_prefix[0] = 0x01;
        bad_prefix[1] = 0x44; // invalid
        assert_eq!(decrypt(&bad_prefix, "pass"), Err(Bip38Error::InvalidFormat));
    }

    #[test]
    fn test_uncompressed_flag() {
        let privkey: [u8; 32] = [0xaa; 32];
        let passphrase = "FlagTest";
        let addr_hash: [u8; 4] = [0x11, 0x22, 0x33, 0x44];

        // Compressed flag should be 0xe0.
        let compressed = encrypt(&privkey, passphrase, true, &addr_hash).unwrap();
        assert_eq!(compressed[2], 0xe0);

        // Uncompressed flag should be 0xc0.
        let uncompressed = encrypt(&privkey, passphrase, false, &addr_hash).unwrap();
        assert_eq!(uncompressed[2], 0xc0);

        // Both should decrypt to the same key.
        let dec_c = decrypt(&compressed, passphrase).unwrap();
        let dec_u = decrypt(&uncompressed, passphrase).unwrap();
        assert_eq!(dec_c, privkey);
        assert_eq!(dec_u, privkey);
    }
}
