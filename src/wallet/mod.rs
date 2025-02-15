// pub mod common;
pub mod web;

use async_trait::async_trait;
use bitcoin::{Transaction, TxOut};
use crate::types::AtomicalsTx;
use crate::errors::Result;

#[async_trait(?Send)]
pub trait WalletProvider {
    async fn get_public_key(&self) -> Result<String>;
    async fn get_address(&self) -> Result<String>;
    async fn get_network(&self) -> Result<String>;
    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<Transaction>;
    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String>;
}
