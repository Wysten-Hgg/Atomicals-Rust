use crate::wallet::WalletProvider;
use crate::types::AtomicalsTx;
use async_trait::async_trait;
use bitcoin::{Transaction, TxOut};
use std::error::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Window};

#[wasm_bindgen]
extern "C" {
    type WizzWallet;

    #[wasm_bindgen(js_namespace = window)]
    fn wizz() -> WizzWallet;

    #[wasm_bindgen(method, js_name = "getPublicKey")]
    fn get_public_key(this: &WizzWallet) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "getAddress")]
    fn get_address(this: &WizzWallet) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "signTransaction")]
    fn sign_transaction(this: &WizzWallet, tx: &str, inputs: &JsValue) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "broadcastTransaction")]
    fn broadcast_transaction(this: &WizzWallet, raw_tx: &str) -> js_sys::Promise;
}

pub struct WizzProvider {
    window: Window,
}

impl WizzProvider {
    pub fn new() -> Option<Self> {
        window().map(|window| Self { window })
    }

    fn get_wizz(&self) -> Result<WizzWallet, Box<dyn Error>> {
        if let Some(wizz) = js_sys::Reflect::get(&self.window, &"wizz".into()).ok() {
            Ok(wizz.unchecked_into())
        } else {
            Err("Wizz wallet not found".into())
        }
    }
}

#[async_trait]
impl WalletProvider for WizzProvider {
    async fn get_public_key(&self) -> Result<String, Box<dyn Error>> {
        let wizz = self.get_wizz()?;
        let promise = wizz.get_public_key();
        let result = JsFuture::from(promise).await?;
        Ok(result.as_string().unwrap_or_default())
    }

    async fn get_address(&self) -> Result<String, Box<dyn Error>> {
        let wizz = self.get_wizz()?;
        let promise = wizz.get_address();
        let result = JsFuture::from(promise).await?;
        Ok(result.as_string().unwrap_or_default())
    }

    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<AtomicalsTx, Box<dyn Error>> {
        let wizz = self.get_wizz()?;
        
        // Convert transaction to hex
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        
        // Convert inputs to JsValue
        let inputs = serde_wasm_bindgen::to_value(&input_txouts)?;
        
        let promise = wizz.sign_transaction(&tx_hex, &inputs);
        let result = JsFuture::from(promise).await?;
        
        // Parse signed transaction
        let signed_tx_hex = result.as_string().ok_or("Invalid signed transaction")?;
        let signed_tx: Transaction = bitcoin::consensus::encode::deserialize_hex(&signed_tx_hex)?;
        
        Ok(AtomicalsTx::new(signed_tx, input_txouts.to_vec()))
    }

    async fn broadcast_transaction(&self, tx: AtomicalsTx) -> Result<String, Box<dyn Error>> {
        let wizz = self.get_wizz()?;
        
        // Convert transaction to hex
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx.raw_tx);
        
        let promise = wizz.broadcast_transaction(&tx_hex);
        let result = JsFuture::from(promise).await?;
        
        Ok(result.as_string().unwrap_or_default())
    }
}
