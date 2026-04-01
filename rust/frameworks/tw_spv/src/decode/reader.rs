// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::error::{SpvError, SpvResult};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;
use tw_hash::H256;

/// A cursor-based deserializer for Bitcoin wire format.
pub struct Reader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Reader<'a> {
    /// Creates a new `Reader` over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Reader {
            cursor: Cursor::new(data),
        }
    }

    /// Reads a single `u8`.
    pub fn read_u8(&mut self) -> SpvResult<u8> {
        self.cursor.read_u8().map_err(|_| SpvError::UnexpectedEof)
    }

    /// Reads a little-endian `u16`.
    pub fn read_u16_le(&mut self) -> SpvResult<u16> {
        self.cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| SpvError::UnexpectedEof)
    }

    /// Reads a little-endian `u32`.
    pub fn read_u32_le(&mut self) -> SpvResult<u32> {
        self.cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| SpvError::UnexpectedEof)
    }

    /// Reads a little-endian `i32`.
    pub fn read_i32_le(&mut self) -> SpvResult<i32> {
        self.cursor
            .read_i32::<LittleEndian>()
            .map_err(|_| SpvError::UnexpectedEof)
    }

    /// Reads a little-endian `u64`.
    pub fn read_u64_le(&mut self) -> SpvResult<u64> {
        self.cursor
            .read_u64::<LittleEndian>()
            .map_err(|_| SpvError::UnexpectedEof)
    }

    /// Reads a little-endian `i64`.
    pub fn read_i64_le(&mut self) -> SpvResult<i64> {
        self.cursor
            .read_i64::<LittleEndian>()
            .map_err(|_| SpvError::UnexpectedEof)
    }

    /// Reads exactly `len` bytes.
    pub fn read_bytes(&mut self, len: usize) -> SpvResult<Vec<u8>> {
        let pos = self.cursor.position() as usize;
        let inner = self.cursor.get_ref();
        if pos + len > inner.len() {
            return Err(SpvError::UnexpectedEof);
        }
        let bytes = inner[pos..pos + len].to_vec();
        self.cursor.set_position((pos + len) as u64);
        Ok(bytes)
    }

    /// Reads a 32-byte hash (`H256`).
    pub fn read_h256(&mut self) -> SpvResult<H256> {
        let bytes = self.read_bytes(32)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(H256::from(arr))
    }

    /// Reads a Bitcoin compact integer (variable-length integer).
    ///
    /// The encoding matches `CompactInteger` in `tw_utxo`:
    /// - `0x00..=0xFC`: 1 byte
    /// - `0xFD`: prefix + 2-byte LE u16
    /// - `0xFE`: prefix + 4-byte LE u32
    /// - `0xFF`: prefix + 8-byte LE u64
    pub fn read_compact_int(&mut self) -> SpvResult<u64> {
        let first = self.read_u8()?;
        match first {
            0x00..=0xFC => Ok(first as u64),
            0xFD => self.read_u16_le().map(|v| v as u64),
            0xFE => self.read_u32_le().map(|v| v as u64),
            0xFF => self.read_u64_le(),
        }
    }

    /// Reads a variable-length byte vector (compact int length prefix followed by bytes).
    pub fn read_var_bytes(&mut self) -> SpvResult<Vec<u8>> {
        let len = self.read_compact_int()? as usize;
        self.read_bytes(len)
    }

    /// Reads a fixed-length, null-padded string of `len` bytes.
    ///
    /// Trailing null bytes are stripped.
    pub fn read_fixed_string(&mut self, len: usize) -> SpvResult<String> {
        let bytes = self.read_bytes(len)?;
        // Find the first null byte and truncate there.
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec())
            .map_err(|e| SpvError::InvalidData(format!("invalid UTF-8: {}", e)))
    }

    /// Returns the current read position.
    pub fn position(&self) -> usize {
        self.cursor.position() as usize
    }

    /// Returns the number of bytes remaining.
    pub fn remaining(&self) -> usize {
        let pos = self.cursor.position() as usize;
        let len = self.cursor.get_ref().len();
        len.saturating_sub(pos)
    }

    /// Returns `true` if there are no bytes remaining.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8() {
        let data = [0x42];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u8().unwrap(), 0x42);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_read_u16_le() {
        let data = [0x01, 0x02];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u16_le().unwrap(), 0x0201);
    }

    #[test]
    fn test_read_u32_le() {
        let data = [0x78, 0x56, 0x34, 0x12];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u32_le().unwrap(), 0x12345678);
    }

    #[test]
    fn test_read_i32_le() {
        // -1 in little-endian i32
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_i32_le().unwrap(), -1);
    }

    #[test]
    fn test_read_u64_le() {
        let data = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_u64_le().unwrap(), 0x8000000000000001);
    }

    #[test]
    fn test_read_i64_le() {
        // -1 in little-endian i64
        let data = [0xFF; 8];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_i64_le().unwrap(), -1);
    }

    #[test]
    fn test_read_bytes() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_bytes(3).unwrap(), vec![0x01, 0x02, 0x03]);
        assert_eq!(reader.remaining(), 2);
        assert_eq!(reader.read_bytes(2).unwrap(), vec![0x04, 0x05]);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_eof_error() {
        let data = [0x01];
        let mut reader = Reader::new(&data);
        // Try to read 2 bytes from 1-byte input.
        assert_eq!(reader.read_u16_le(), Err(SpvError::UnexpectedEof));
    }

    #[test]
    fn test_eof_error_read_bytes() {
        let data = [0x01, 0x02];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_bytes(3), Err(SpvError::UnexpectedEof));
    }

    #[test]
    fn test_read_h256() {
        let mut data = [0u8; 32];
        data[0] = 0xAB;
        data[31] = 0xCD;
        let mut reader = Reader::new(&data);
        let hash = reader.read_h256().unwrap();
        assert_eq!(hash.as_slice()[0], 0xAB);
        assert_eq!(hash.as_slice()[31], 0xCD);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_compact_int_one_byte() {
        let data = [0x42];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_compact_int().unwrap(), 0x42);
    }

    #[test]
    fn test_compact_int_two_bytes() {
        // 0xFD prefix + 0xFD 0x00 (little-endian 253)
        let data = [0xFD, 0xFD, 0x00];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_compact_int().unwrap(), 0xFD);
    }

    #[test]
    fn test_compact_int_four_bytes() {
        // 0xFE prefix + 4-byte LE 0x00010000
        let data = [0xFE, 0x00, 0x00, 0x01, 0x00];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_compact_int().unwrap(), 0x00010000);
    }

    #[test]
    fn test_compact_int_eight_bytes() {
        // 0xFF prefix + 8-byte LE 0x0000000100000000
        let data = [0xFF, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_compact_int().unwrap(), 0x0000000100000000);
    }

    #[test]
    fn test_read_var_bytes() {
        let data = [0x03, 0xAA, 0xBB, 0xCC];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_var_bytes().unwrap(), vec![0xAA, 0xBB, 0xCC]);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_read_fixed_string() {
        // "version\0\0\0\0\0" — 12-byte null-padded command
        let mut data = [0u8; 12];
        data[..7].copy_from_slice(b"version");
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_fixed_string(12).unwrap(), "version");
        assert!(reader.is_empty());
    }

    #[test]
    fn test_read_fixed_string_full() {
        // All 12 bytes used, no null padding.
        let data = b"abcdefghijkl";
        let mut reader = Reader::new(data);
        assert_eq!(reader.read_fixed_string(12).unwrap(), "abcdefghijkl");
    }

    #[test]
    fn test_position_and_remaining() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.remaining(), 4);
        assert!(!reader.is_empty());

        reader.read_u8().unwrap();
        assert_eq!(reader.position(), 1);
        assert_eq!(reader.remaining(), 3);

        reader.read_u16_le().unwrap();
        assert_eq!(reader.position(), 3);
        assert_eq!(reader.remaining(), 1);
    }
}
