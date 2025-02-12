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
    type UniSat;

    #[wasm_bindgen(js_namespace = window)]
    fn unisat() -> UniSat;

    #[wasm_bindgen(method, js_name = "getPublicKey")]
    fn get_public_key(this: &UniSat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "getAddress")]
    fn get_address(this: &UniSat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "signTransaction")]
    fn sign_transaction(this: &UniSat, tx: &str, inputs: &JsValue) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "pushTransaction")]
    fn push_transaction(this: &UniSat, raw_tx: &str) -> js_sys::Promise;
}

pub struct UnisatProvider {
    window: Window,
}

impl UnisatProvider {
    pub fn new() -> Option<Self> {
        window().map(|window| Self { window })
    }

    fn get_unisat(&self) -> Result<UniSat, Box<dyn Error>> {
        if let Some(unisat) = js_sys::Reflect::get(&self.window, &"unisat".into()).ok() {
            Ok(unisat.unchecked_into())
        } else {
            Err("UniSat wallet not found".into())
        }
    }
}

#[async_trait]
impl WalletProvider for UnisatProvider {
    async fn get_public_key(&self) -> Result<String, Box<dyn Error>> {
        let unisat = self.get_unisat()?;
        let promise = unisat.get_public_key();
        let result = JsFuture::from(promise).await?;
        Ok(result.as_string().unwrap_or_default())
    }

    async fn get_address(&self) -> Result<String, Box<dyn Error>> {
        let unisat = self.get_unisat()?;
        let promise = unisat.get_address();
        let result = JsFuture::from(promise).await?;
        Ok(result.as_string().unwrap_or_default())
    }

    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<AtomicalsTx, Box<dyn Error>> {
        let unisat = self.get_unisat()?;
        
        // Convert transaction to hex
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        
        // Convert inputs to JsValue
        let inputs = serde_wasm_bindgen::to_value(&input_txouts)?;
        
        let promise = unisat.sign_transaction(&tx_hex, &inputs);
        let result = JsFuture::from(promise).await?;
        
        // Parse signed transaction
        let signed_tx_hex = result.as_string().ok_or("Invalid signed transaction")?;
        let signed_tx: Transaction = bitcoin::consensus::encode::deserialize_hex(&signed_tx_hex)?;
        
        Ok(AtomicalsTx::new(signed_tx, input_txouts.to_vec()))
    }

    async fn broadcast_transaction(&self, tx: AtomicalsTx) -> Result<String, Box<dyn Error>> {
        let unisat = self.get_unisat()?;
        
        // Convert transaction to hex
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx.raw_tx);
        
        let promise = unisat.push_transaction(&tx_hex);
        let result = JsFuture::from(promise).await?;
        
        Ok(result.as_string().unwrap_or_default())
    }
}
