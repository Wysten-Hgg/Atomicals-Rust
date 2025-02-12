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

    #[wasm_bindgen(method, js_name = "getPublicKey")]
    fn get_public_key(this: &UniSat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "getAddress")]
    fn get_address(this: &UniSat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "signTransaction")]
    fn sign_transaction(this: &UniSat, tx: &str, inputs: &JsValue) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "pushTransaction")]
    fn push_transaction(this: &UniSat, raw_tx: &str) -> js_sys::Promise;
}

#[derive(Clone)]
pub struct UnisatProvider {
    window: Window,
}

impl UnisatProvider {
    pub fn new() -> Option<Self> {
        window().map(|w| Self { window: w })
    }

    fn get_unisat(&self) -> Result<UniSat, Error> {
        self.window
            .get("unisat")
            .ok_or_else(|| Error::WalletError("Unisat not found".to_string()))
            .and_then(|val| val.dyn_into::<UniSat>().map_err(|_| Error::WalletError("Invalid Unisat instance".to_string())))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl WalletProvider for UnisatProvider {
    async fn get_public_key(&self) -> Result<String, Error> {
        let unisat = self.get_unisat()?;
        let promise = unisat.get_public_key();
        let result = JsFuture::from(promise).await
            .map_err(|e| Error::WalletError(format!("Failed to get public key: {:?}", e)))?;
        let public_key = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid public key".to_string()))?;
        Ok(public_key)
    }

    async fn get_address(&self) -> Result<String, Error> {
        let unisat = self.get_unisat()?;
        let promise = unisat.get_address();
        let result = JsFuture::from(promise).await
            .map_err(|e| Error::WalletError(format!("Failed to get address: {:?}", e)))?;
        let address = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid address".to_string()))?;
        Ok(address)
    }

    async fn sign_transaction(&self, tx: Transaction, input_txouts: &[TxOut]) -> Result<AtomicalsTx, Error> {
        let unisat = self.get_unisat()?;
        
        // Convert transaction to hex
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        
        // Convert inputs to JsValue
        let inputs = serde_wasm_bindgen::to_value(&input_txouts)?;
        
        let promise = unisat.sign_transaction(&tx_hex, &inputs);
        let result = JsFuture::from(promise).await
            .map_err(|e| Error::WalletError(format!("Failed to sign transaction: {:?}", e)))?;
        
        // Parse signed transaction
        let signed_tx_hex = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid signed transaction".to_string()))?;
        let signed_tx: Transaction = bitcoin::consensus::encode::deserialize(&hex::decode(&signed_tx_hex)?)?;
        
        Ok(AtomicalsTx::new(signed_tx, input_txouts.to_vec()))
    }

    async fn broadcast_transaction(&self, tx: AtomicalsTx) -> Result<String, Error> {
        let unisat = self.get_unisat()?;
        
        // Convert transaction to hex
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx.raw_tx);
        
        let promise = unisat.push_transaction(&tx_hex);
        let result = JsFuture::from(promise).await
            .map_err(|e| Error::WalletError(format!("Failed to broadcast transaction: {:?}", e)))?;
        
        let txid = result.as_string()
            .ok_or_else(|| Error::WalletError("Invalid transaction ID".to_string()))?;
        Ok(txid)
    }
}
