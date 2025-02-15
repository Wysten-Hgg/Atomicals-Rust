use crate::errors::{Error, Result};
use crate::types::AtomicalsTx;
use crate::wallet::WalletProvider;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Window};
use bitcoin::{Transaction, TxOut};
use hex;
use serde_wasm_bindgen;
use async_trait::async_trait;

#[wasm_bindgen]
extern "C" {
    type UniSat;

    #[wasm_bindgen(js_namespace = window)]
    fn unisat() -> UniSat;

    #[wasm_bindgen(method, js_name = "getAccounts")]
    fn get_accounts(this: &UniSat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "signTransaction")]
    fn sign_transaction(this: &UniSat, tx: &str) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "sendTransaction")]
    fn send_transaction(this: &UniSat, tx: &str) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "getNetwork")]
    fn get_network(this: &UniSat) -> js_sys::Promise;
}

#[derive(Clone)]
pub struct UnisatProvider {
    window: web_sys::Window,
}

// 实现 Send 和 Sync，因为我们知道 Window 在 WASM 环境中是线程安全的
unsafe impl Send for UnisatProvider {}
unsafe impl Sync for UnisatProvider {}

impl UnisatProvider {
    pub fn new() -> Result<Self> {
        let window = web_sys::window()
            .ok_or_else(|| Error::WasmError("Failed to get window".to_string()))?;
        Ok(Self { window })
    }

    fn get_unisat(&self) -> Result<UniSat> {
        let unisat = js_sys::Reflect::get(&self.window, &"unisat".into())
            .map_err(|e| Error::WasmError(format!("Failed to get unisat: {:?}", e)))?;
        Ok(unisat.unchecked_into())
    }

    pub async fn get_public_key(&self) -> Result<String> {
        let unisat = self.get_unisat()?;
        let accounts = JsFuture::from(unisat.get_accounts())
            .await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts: Vec<String> = serde_wasm_bindgen::from_value(accounts)
            .map_err(|e| Error::WasmError(format!("Failed to parse accounts: {:?}", e)))?;
        accounts.first()
            .cloned()
            .ok_or_else(|| Error::WasmError("No accounts found".to_string()))
    }

    pub async fn get_address(&self) -> Result<String> {
        let unisat = self.get_unisat()?;
        let accounts = JsFuture::from(unisat.get_accounts())
            .await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts: Vec<String> = serde_wasm_bindgen::from_value(accounts)
            .map_err(|e| Error::WasmError(format!("Failed to parse accounts: {:?}", e)))?;
        accounts.first()
            .cloned()
            .ok_or_else(|| Error::WasmError("No accounts found".to_string()))
    }

    pub async fn get_network(&self) -> Result<String> {
        let unisat = self.get_unisat()?;
        let network = JsFuture::from(unisat.get_network())
            .await
            .map_err(|e| Error::WasmError(format!("Failed to get network: {:?}", e)))?;
        let network: String = serde_wasm_bindgen::from_value(network)
            .map_err(|e| Error::WasmError(format!("Failed to parse network: {:?}", e)))?;
        Ok(network)
    }

    pub async fn sign_transaction(&self, tx: Transaction, _utxos: &[TxOut]) -> Result<Transaction> {
        let unisat = self.get_unisat()?;
        let tx_hex = hex::encode(bitcoin::consensus::serialize(&tx));
        
        let signed_tx = JsFuture::from(unisat.sign_transaction(&tx_hex))
            .await
            .map_err(|e| Error::WasmError(format!("Failed to sign transaction: {:?}", e)))?;
        
        let signed_tx_hex: String = serde_wasm_bindgen::from_value(signed_tx)
            .map_err(|e| Error::WasmError(format!("Failed to parse signed transaction: {:?}", e)))?;
        
        let signed_tx_bytes = hex::decode(signed_tx_hex)
            .map_err(|e| Error::WasmError(format!("Failed to decode signed transaction: {:?}", e)))?;
        
        bitcoin::consensus::deserialize(&signed_tx_bytes)
            .map_err(|e| Error::WasmError(format!("Failed to deserialize signed transaction: {:?}", e)))
    }

    pub async fn broadcast_transaction(&self, tx: Transaction) -> Result<String> {
        let unisat = self.get_unisat()?;
        let tx_hex = hex::encode(bitcoin::consensus::serialize(&tx));
        
        let txid = JsFuture::from(unisat.send_transaction(&tx_hex))
            .await
            .map_err(|e| Error::WasmError(format!("Failed to broadcast transaction: {:?}", e)))?;
        
        serde_wasm_bindgen::from_value(txid)
            .map_err(|e| Error::WasmError(format!("Failed to parse txid: {:?}", e)))
    }
}

#[async_trait(?Send)]
impl WalletProvider for UnisatProvider {
    async fn get_public_key(&self) -> Result<String> {
        self.get_public_key().await
    }

    async fn get_address(&self) -> Result<String> {
        self.get_address().await
    }

    async fn get_network(&self) -> Result<String> {
        self.get_network().await
    }

    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<Transaction> {
        self.sign_transaction(tx, input_txouts).await
    }

    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String> {
        self.broadcast_transaction(tx).await
    }
}
