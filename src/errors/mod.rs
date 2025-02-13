use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    
    #[error("Invalid ticker: {0}")]
    InvalidTicker(String),
    
    #[error("Invalid bitwork: {0}")]
    InvalidBitwork(String),
    
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    
    #[error("Signing error: {0}")]
    SigningError(String),
    
    #[error("Broadcast error: {0}")]
    BroadcastError(String),
    
    #[error("Wallet error: {0}")]
    WalletError(String),
    
    #[error("Mining timeout: {0}")]
    MiningTimeout(String),
    
    #[error("Mining error: {0}")]
    MiningError(String),
    
    #[error("Bitcoin error: {0}")]
    BitcoinError(#[from] bitcoin::Error),
    
    #[error("WASM error: {0}")]
    WasmError(String),
    
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Generic error: {0}")]
    Generic(Box<dyn StdError + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Box<dyn StdError + Send + Sync>> for Error {
    fn from(err: Box<dyn StdError + Send + Sync>) -> Self {
        Error::Generic(err)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::Generic(Box::new(e))
    }
}

impl From<bitcoin::consensus::encode::Error> for Error {
    fn from(e: bitcoin::consensus::encode::Error) -> Self {
        Error::Generic(Box::new(e))
    }
}
