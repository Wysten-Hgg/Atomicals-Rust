use std::fmt;
use std::error::Error as StdError;

#[derive(Debug)]
pub enum Error {
    // General errors
    InvalidAmount(String),
    InvalidTicker(String),
    InvalidBitwork(String),
    
    // Wallet errors
    WalletNotFound(String),
    SigningError(String),
    BroadcastError(String),
    
    // Mining errors
    MiningTimeout(String),
    MiningError(String),
    
    // Bitcoin errors
    BitcoinError(bitcoin::Error),
    
    // External errors
    WasmError(String),
    SerdeError(String),
    IoError(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            Error::InvalidTicker(msg) => write!(f, "Invalid ticker: {}", msg),
            Error::InvalidBitwork(msg) => write!(f, "Invalid bitwork: {}", msg),
            Error::WalletNotFound(msg) => write!(f, "Wallet not found: {}", msg),
            Error::SigningError(msg) => write!(f, "Signing error: {}", msg),
            Error::BroadcastError(msg) => write!(f, "Broadcast error: {}", msg),
            Error::MiningTimeout(msg) => write!(f, "Mining timeout: {}", msg),
            Error::MiningError(msg) => write!(f, "Mining error: {}", msg),
            Error::BitcoinError(e) => write!(f, "Bitcoin error: {}", e),
            Error::WasmError(msg) => write!(f, "WASM error: {}", msg),
            Error::SerdeError(msg) => write!(f, "Serialization error: {}", msg),
            Error::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::BitcoinError(e) => Some(e),
            Error::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<bitcoin::Error> for Error {
    fn from(err: bitcoin::Error) -> Self {
        Error::BitcoinError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
