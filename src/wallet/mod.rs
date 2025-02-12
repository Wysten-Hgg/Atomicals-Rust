// pub mod common;
pub mod web;

use async_trait::async_trait;
use bitcoin::{Transaction, TxOut};
use crate::types::AtomicalsTx;
use crate::errors::Result;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait WalletProvider {
    async fn get_public_key(&self) -> Result<String>;
    async fn get_address(&self) -> Result<String>;
    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<Transaction>;
    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String>;
}
