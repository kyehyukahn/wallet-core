use std::fmt;

/// Errors related to persistence operations.
#[derive(Debug)]
pub enum PersistenceError {
    StorageError(String),
    DeserializationError(String),
    AtomicWriteFailed(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::StorageError(msg) => write!(f, "Storage error: {msg}"),
            PersistenceError::DeserializationError(msg) => {
                write!(f, "Deserialization error: {msg}")
            },
            PersistenceError::AtomicWriteFailed(msg) => {
                write!(f, "Atomic write failed: {msg}")
            },
        }
    }
}

impl std::error::Error for PersistenceError {}

/// Top-level wallet errors.
#[derive(Debug)]
pub enum WalletError {
    AddressDerivation(String),
    TransactionNotFound,
    UtxoNotFound,
    InsufficientBalance { available: u64, required: u64 },
    Persistence(PersistenceError),
    InvalidState(String),
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::AddressDerivation(msg) => write!(f, "Address derivation error: {msg}"),
            WalletError::TransactionNotFound => write!(f, "Transaction not found"),
            WalletError::UtxoNotFound => write!(f, "UTXO not found"),
            WalletError::InsufficientBalance {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient balance: available {available}, required {required}"
                )
            },
            WalletError::Persistence(e) => write!(f, "Persistence error: {e}"),
            WalletError::InvalidState(msg) => write!(f, "Invalid state: {msg}"),
        }
    }
}

impl std::error::Error for WalletError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WalletError::Persistence(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PersistenceError> for WalletError {
    fn from(e: PersistenceError) -> Self {
        WalletError::Persistence(e)
    }
}

/// Convenience type alias for wallet operations.
pub type WalletResult<T> = Result<T, WalletError>;
