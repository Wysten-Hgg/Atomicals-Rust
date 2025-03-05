use std::fmt;
use std::error::Error as StdError;
use bitcoin::consensus;
use wasm_bindgen::JsValue;
use serde_wasm_bindgen;
use serde_json;

#[derive(Debug)]
pub enum Error {
    NetworkError(String),
    AddressError(String),
    TransactionError(String),
    PsbtError(String),
    MiningError(String),
    WalletError(String),
    SerializationError(String),
    DeserializationError(String),
    WorkerError(String),
    InvalidAmount(String),
    InvalidTicker(String),
    InvalidBitwork(String),
    WasmError(String),
    IoError(std::io::Error),
    SerdeError(serde_wasm_bindgen::Error),
    HexError(String),
    SerdeJsonError(serde_json::Error),
    AsyncError(String),
    InvalidInput(String),
    DatabaseError(String),
    RealmNameInvalid(String),
    ParentRealmNotFound(String),
    ContainerNotFound(String),
    ParentOwnerInvalid(String),
    OwnershipError(String),
    AtomicalNotFound(String),
    ScriptError(String),
    NotImplemented(String),
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Error::AddressError(msg) => write!(f, "Address error: {}", msg),
            Error::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            Error::PsbtError(msg) => write!(f, "PSBT error: {}", msg),
            Error::MiningError(msg) => write!(f, "Mining error: {}", msg),
            Error::WalletError(msg) => write!(f, "Wallet error: {}", msg),
            Error::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Error::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            Error::WorkerError(msg) => write!(f, "Worker error: {}", msg),
            Error::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            Error::InvalidTicker(msg) => write!(f, "Invalid ticker: {}", msg),
            Error::InvalidBitwork(msg) => write!(f, "Invalid bitwork: {}", msg),
            Error::WasmError(msg) => write!(f, "WASM error: {}", msg),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::SerdeError(e) => write!(f, "Serde error: {}", e),
            Error::HexError(e) => write!(f, "Hex error: {}", e),
            Error::SerdeJsonError(e) => write!(f, "JSON error: {}", e),
            Error::AsyncError(msg) => write!(f, "Async error: {}", msg),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Error::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Error::RealmNameInvalid(msg) => write!(f, "Invalid realm name: {}", msg),
            Error::ParentRealmNotFound(msg) => write!(f, "Parent realm not found: {}", msg),
            Error::ContainerNotFound(msg) => write!(f, "Container not found: {}", msg),
            Error::ParentOwnerInvalid(msg) => write!(f, "Invalid parent owner: {}", msg),
            Error::OwnershipError(msg) => write!(f, "Ownership error: {}", msg),
            Error::AtomicalNotFound(msg) => write!(f, "Atomical not found: {}", msg),
            Error::ScriptError(msg) => write!(f, "Script error: {}", msg),
            Error::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            Error::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::IoError(e) => Some(e),
            Error::SerdeError(e) => Some(e),
            Error::SerdeJsonError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<consensus::encode::Error> for Error {
    fn from(e: consensus::encode::Error) -> Self {
        Error::SerializationError(e.to_string())
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::HexError(e.to_string())
    }
}

impl From<serde_wasm_bindgen::Error> for Error {
    fn from(e: serde_wasm_bindgen::Error) -> Self {
        Error::SerdeError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::SerdeJsonError(e)
    }
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Error::WasmError(value.as_string().unwrap_or_else(|| "Unknown JS error".to_string()))
    }
}

impl From<Error> for JsValue {
    fn from(error: Error) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

// 添加 script::Error 的转换
impl From<crate::utils::script::Error> for Error {
    fn from(e: crate::utils::script::Error) -> Self {
        Error::SerializationError(e.to_string())
    }
}

// 添加 TaprootBuilderError 的转换
impl From<bitcoin::taproot::TaprootBuilderError> for Error {
    fn from(e: bitcoin::taproot::TaprootBuilderError) -> Self {
        Error::TransactionError(e.to_string())
    }
}

// 添加 TaprootBuilder 的转换
impl From<bitcoin::taproot::TaprootBuilder> for Error {
    fn from(e: bitcoin::taproot::TaprootBuilder) -> Self {
        Error::TransactionError("Taproot builder error".to_string())
    }
}

// 添加 bitcoin::address::Error 的转换
impl From<bitcoin::address::Error> for Error {
    fn from(e: bitcoin::address::Error) -> Self {
        Error::AddressError(e.to_string())
    }
}
