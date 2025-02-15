// pub mod common;
pub mod web;

use async_trait::async_trait;
use bitcoin::{Network, PublicKey, Transaction, TxOut};
use bitcoin::psbt::Psbt;
use crate::types::AtomicalsTx;
use crate::errors::{Result, Error};

#[async_trait(?Send)]
pub trait WalletProvider {
    async fn get_network(&self) -> Result<Network>;
    async fn get_public_key(&self) -> Result<PublicKey>;
    async fn get_address(&self) -> Result<String>;
    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<Transaction>;
    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String>;
    
    // 新增方法
    async fn sign_psbt(&self, psbt: Psbt) -> Result<Psbt>;
    
    async fn sign_atomicals_transactions(&self, commit_psbt: Psbt, reveal_psbt: Psbt) -> Result<(Transaction, Transaction)> {
        Err(Error::WalletError("sign_atomicals_transactions not implemented".to_string()))
    }
}
