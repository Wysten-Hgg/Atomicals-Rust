use crate::errors::{Error, Result};
use crate::wallet::WalletProvider;
use async_trait::async_trait;
use bitcoin::{Transaction, TxOut, Network, PublicKey};
use bitcoin::psbt::Psbt;
use wasm_bindgen::prelude::*;
use js_sys::{Function, Object, Promise, Reflect, Array};
use serde_wasm_bindgen::{to_value, from_value};
use std::str::FromStr;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[wasm_bindgen]
#[derive(Debug)]
pub struct UnisatProvider {
    wallet: Object,
}

#[wasm_bindgen]
impl UnisatProvider {
    #[wasm_bindgen(constructor)]
    pub fn try_new() -> std::result::Result<UnisatProvider, JsValue> {
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("Window not available"))?;
        let unisat = Reflect::get(&window, &"unisat".into())
            .map_err(|_| JsValue::from_str("Unisat wallet not found"))?;
        
        Ok(UnisatProvider {
            wallet: unisat.unchecked_into(),
        })
    }
}

#[async_trait(?Send)]
impl WalletProvider for UnisatProvider {
    async fn get_network(&self) -> Result<Network> {
        let network = self.call_wallet_method("getNetwork", &[])?;
        let network_str: String = from_value(network)?;
        
        match network_str.as_str() {
            "mainnet" => Ok(Network::Bitcoin),
            "testnet" => Ok(Network::Testnet),
            _ => Err(Error::NetworkError(format!("Unsupported network: {}", network_str))),
        }
    }

    async fn get_public_key(&self) -> Result<PublicKey> {
        let pubkey = self.call_wallet_method("getPublicKey", &[])?;
        let pubkey_str: String = from_value(pubkey)?;
        
        PublicKey::from_str(&pubkey_str)
            .map_err(|e| Error::WalletError(format!("Invalid public key: {}", e)))
    }

    async fn get_address(&self) -> Result<String> {
        let address = self.call_wallet_method("getAddress", &[])?;
        from_value(address)
            .map_err(|e| Error::WalletError(format!("Failed to get address: {}", e)))
    }

    async fn sign_transaction(&self, tx: Transaction, outputs: &[TxOut]) -> Result<Transaction> {
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        
        let args = vec![
            to_value(&tx_hex)?,
            to_value(&outputs)?,
        ];
        
        let result = self.call_wallet_method("signTransaction", &args)?;
        
        let signed_tx_hex: String = from_value(result)?;
        let tx_bytes = hex::decode(signed_tx_hex)?;
        let signed_tx = bitcoin::consensus::encode::deserialize(&tx_bytes)?;
        
        Ok(signed_tx)
    }

    async fn broadcast_transaction(&self, tx: Transaction) -> Result<String> {
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        let result = self.call_wallet_method("broadcastTransaction", &[to_value(&tx_hex)?])?;
        
        from_value(result)
            .map_err(|e| Error::WalletError(format!("Failed to broadcast transaction: {}", e)))
    }

    async fn sign_psbt(&self, psbt: Psbt) -> Result<Psbt> {
        let psbt_bytes = psbt.serialize();
        let psbt_base64 = BASE64.encode(psbt_bytes);
        
        let result = self.call_wallet_method("signPsbt", &[to_value(&psbt_base64)?])?;
        
        let signed_psbt_str: String = from_value(result)?;
        let signed_psbt_bytes = BASE64.decode(signed_psbt_str.as_bytes())
            .map_err(|e| Error::WalletError(format!("Failed to decode base64 PSBT: {}", e)))?;
        
        Psbt::deserialize(&signed_psbt_bytes)
            .map_err(|e| Error::WalletError(format!("Failed to parse signed PSBT: {}", e)))
    }
}

impl UnisatProvider {
    fn call_wallet_method(&self, method: &str, args: &[JsValue]) -> Result<JsValue> {
        let method_fn = Reflect::get(&self.wallet, &method.into())
            .map_err(|_| Error::WalletError(format!("Method {} not found", method)))?;
        
        let method_fn: Function = method_fn.unchecked_into();
        let args_array = Array::new();
        for arg in args {
            args_array.push(arg);
        }
        
        method_fn.apply(&self.wallet, &args_array)
            .map_err(|e| Error::WalletError(format!("Failed to call {}: {:?}", method, e)))
    }
}
