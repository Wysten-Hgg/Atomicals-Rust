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
        let accounts = JsFuture::from(unisat.get_accounts()).await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts_array = js_sys::Array::from(&accounts);
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts found".to_string()));
        }
        let address = accounts_array.get(0)
            .as_string()
            .ok_or_else(|| Error::WalletError("Invalid address format".to_string()))?;
        Ok(address)
    }

    pub async fn get_address(&self) -> Result<String> {
        let unisat = self.get_unisat()?;
        let accounts = JsFuture::from(unisat.get_accounts()).await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts_array = js_sys::Array::from(&accounts);
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts found".to_string()));
        }
        let address = accounts_array.get(0)
            .as_string()
            .ok_or_else(|| Error::WalletError("Invalid address format".to_string()))?;
        Ok(address)
    }

    async fn sign_transaction(&self, tx: Transaction, _utxos: &[TxOut]) -> Result<Transaction> {
        let unisat = self.get_unisat()?;
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = JsFuture::from(unisat.sign_transaction(&tx_hex)).await
            .map_err(|e| Error::WalletError(format!("Failed to sign transaction: {:?}", e)))?;
        
        let signed_tx_hex = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid signed transaction".to_string()))?;
        let signed_tx: Transaction = bitcoin::consensus::encode::deserialize(&hex::decode(&signed_tx_hex)?)?;
        
        Ok(signed_tx)
    }

    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String> {
        let unisat = self.get_unisat()?;
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = JsFuture::from(unisat.send_transaction(&tx_hex)).await
            .map_err(|e| Error::WalletError(format!("Failed to broadcast transaction: {:?}", e)))?;
        let txid = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid transaction ID".to_string()))?;
        Ok(txid)
    }
}

#[async_trait(?Send)]
impl WalletProvider for UnisatProvider {
    async fn get_public_key(&self) -> Result<String> {
        let unisat = self.get_unisat()?;
        let accounts = JsFuture::from(unisat.get_accounts()).await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts_array = js_sys::Array::from(&accounts);
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts found".to_string()));
        }
        let address = accounts_array.get(0)
            .as_string()
            .ok_or_else(|| Error::WalletError("Invalid address format".to_string()))?;
        Ok(address)
    }

    async fn get_address(&self) -> Result<String> {
        let unisat = self.get_unisat()?;
        let accounts = JsFuture::from(unisat.get_accounts()).await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts_array = js_sys::Array::from(&accounts);
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts found".to_string()));
        }
        let address = accounts_array.get(0)
            .as_string()
            .ok_or_else(|| Error::WalletError("Invalid address format".to_string()))?;
        Ok(address)
    }

    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<Transaction> {
        let unisat = self.get_unisat()?;
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = JsFuture::from(unisat.sign_transaction(&tx_hex)).await
            .map_err(|e| Error::WasmError(format!("Failed to sign transaction: {:?}", e)))?;
        let signed_tx_hex = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid signed transaction format".to_string()))?;
        
        let signed_tx_bytes = hex::decode(&signed_tx_hex)
            .map_err(|e| Error::Generic(Box::new(e)))?;
        let signed_tx = bitcoin::consensus::encode::deserialize(&signed_tx_bytes)
            .map_err(|e| Error::Generic(Box::new(e)))?;
        Ok(signed_tx)
    }

    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String> {
        let unisat = self.get_unisat()?;
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = JsFuture::from(unisat.send_transaction(&tx_hex)).await
            .map_err(|e| Error::WasmError(format!("Failed to broadcast transaction: {:?}", e)))?;
        let txid = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid transaction ID format".to_string()))?;
        Ok(txid)
    }
}
