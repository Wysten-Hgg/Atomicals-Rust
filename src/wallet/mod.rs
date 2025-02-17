// pub mod common;
pub mod web;

use async_trait::async_trait;
use bitcoin::{Network, PublicKey, Transaction, TxOut, Amount, OutPoint};
use bitcoin::psbt::Psbt;
use crate::types::AtomicalsTx;
use crate::errors::{Result, Error};

#[derive(Debug, Clone)]
pub struct Utxo {
    pub outpoint: OutPoint,
    pub txout: TxOut,
    pub height: Option<u32>,
}

#[async_trait(?Send)]
pub trait WalletProvider {
    async fn get_network(&self) -> Result<Network>;
    async fn get_public_key(&self) -> Result<PublicKey>;
    async fn get_address(&self) -> Result<String>;
    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<Transaction>;
    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String>;
    async fn sign_psbt(&self, psbt: Psbt) -> Result<Psbt>;
    
    // UTXO 相关方法
    async fn get_utxos(&self) -> Result<Vec<Utxo>> {
        Err(Error::WalletError("get_utxos not implemented".to_string()))
    }
    
    async fn get_balance(&self) -> Result<Amount> {
        Err(Error::WalletError("get_balance not implemented".to_string()))
    }
    
    // 获取网络费率（satoshis/vbyte）
    async fn get_network_fee_rate(&self) -> Result<f64> {
        Err(Error::WalletError("get_network_fee_rate not implemented".to_string()))
    }
    
    async fn sign_atomicals_transactions(&self, commit_psbt: Psbt, reveal_psbt: Psbt) -> Result<(Transaction, Transaction)> {
        Err(Error::WalletError("sign_atomicals_transactions not implemented".to_string()))
    }
}
