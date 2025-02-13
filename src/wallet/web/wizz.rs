use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Promise, Array};
use web_sys::Window;
use crate::errors::{Error, Result};
use crate::wallet::WalletProvider;
use bitcoin::{Transaction, TxOut};
use async_trait::async_trait;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Wizz")]
    type Wizz;

    #[wasm_bindgen(method, js_name = "getAccounts")]
    fn get_accounts(this: &Wizz) -> Promise;

    #[wasm_bindgen(method, js_name = "signTransaction")]
    fn sign_transaction(this: &Wizz, tx_hex: &str) -> Promise;

    #[wasm_bindgen(method, js_name = "sendTransaction")]
    fn send_transaction(this: &Wizz, tx_hex: &str) -> Promise;
}

#[derive(Clone)]
pub struct WizzProvider {
    window: Window,
}

impl WizzProvider {
    pub fn new() -> Result<Self> {
        let window = web_sys::window()
            .ok_or_else(|| Error::WasmError("Failed to get window".to_string()))?;
        Ok(Self { window })
    }

    fn get_wizz(&self) -> Result<Wizz> {
        let wizz = js_sys::Reflect::get(&self.window, &"wizz".into())
            .map_err(|e| Error::WasmError(format!("Failed to get wizz: {:?}", e)))?;
        Ok(wizz.unchecked_into())
    }
}

#[async_trait(?Send)]
impl WalletProvider for WizzProvider {
    async fn get_public_key(&self) -> Result<String> {
        let wizz = self.get_wizz()?;
        let accounts = JsFuture::from(wizz.get_accounts()).await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts_array = Array::from(&accounts);
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts found".to_string()));
        }
        let address = accounts_array.get(0)
            .as_string()
            .ok_or_else(|| Error::WalletError("Invalid address format".to_string()))?;
        Ok(address)
    }

    async fn get_address(&self) -> Result<String> {
        let wizz = self.get_wizz()?;
        let accounts = JsFuture::from(wizz.get_accounts()).await
            .map_err(|e| Error::WasmError(format!("Failed to get accounts: {:?}", e)))?;
        let accounts_array = Array::from(&accounts);
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts found".to_string()));
        }
        let address = accounts_array.get(0)
            .as_string()
            .ok_or_else(|| Error::WalletError("Invalid address format".to_string()))?;
        Ok(address)
    }

    async fn sign_transaction(&self, tx: Transaction, _input_txouts: &[TxOut]) -> Result<Transaction> {
        let wizz = self.get_wizz()?;
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = JsFuture::from(wizz.sign_transaction(&tx_hex)).await
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
        let wizz = self.get_wizz()?;
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = JsFuture::from(wizz.send_transaction(&tx_hex)).await
            .map_err(|e| Error::WasmError(format!("Failed to broadcast transaction: {:?}", e)))?;
        let txid = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid transaction ID format".to_string()))?;
        Ok(txid)
    }
}
