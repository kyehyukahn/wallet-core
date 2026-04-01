// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use std::fmt;

/// Errors that can occur during SPV message decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpvError {
    /// The input ended before the expected data could be read.
    UnexpectedEof,
    /// The data does not conform to the expected format.
    InvalidData(String),
    /// A checksum in the data did not match the expected value.
    ChecksumMismatch,
    /// The message exceeds the maximum allowed size.
    MessageTooLarge,
    /// The command string is not recognized.
    UnknownCommand(String),
}

impl fmt::Display for SpvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpvError::UnexpectedEof => write!(f, "unexpected end of input"),
            SpvError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            SpvError::ChecksumMismatch => write!(f, "checksum mismatch"),
            SpvError::MessageTooLarge => write!(f, "message too large"),
            SpvError::UnknownCommand(cmd) => write!(f, "unknown command: {}", cmd),
        }
    }
}

impl std::error::Error for SpvError {}

/// A specialized `Result` type for SPV operations.
pub type SpvResult<T> = Result<T, SpvError>;
