// pub mod common;
pub mod web;

use async_trait::async_trait;
use bitcoin::{Transaction, TxOut};
use crate::types::AtomicalsTx;
use std::error::Error;

#[async_trait]
pub trait WalletProvider: Send + Sync {
    async fn get_public_key(&self) -> Result<String, Box<dyn Error>>;
    async fn get_address(&self) -> Result<String, Box<dyn Error>>;
    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<AtomicalsTx, Box<dyn Error>>;
    async fn broadcast_transaction(&self, tx: AtomicalsTx) -> Result<String, Box<dyn Error>>;
}
