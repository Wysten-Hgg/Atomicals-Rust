use crate::errors::{Error, Result};
use crate::wallet::WalletProvider;
use crate::types::atomicals::{AtomicalInfo, LocationInfo, AtomicalState};
use async_trait::async_trait;
use bitcoin::{Transaction, TxOut, Network, PublicKey};
use bitcoin::psbt::Psbt;
use wasm_bindgen::prelude::*;
use js_sys::{Function, Object, Promise, Reflect, Array};
use serde_wasm_bindgen::{to_value, from_value};
use serde::{Deserialize};
use serde_json::Value;
use std::str::FromStr;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest;

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

    async fn get_atomical_by_id(&self, atomical_id: &str) -> Result<AtomicalInfo> {
        // 使用 UniSat 的 API 获取 Atomical 信息
        let url = format!(
            "https://testnet.unisat.io/api/v1/atomical/{}",
            atomical_id
        );

        let response = reqwest::get(&url).await
            .map_err(|e| Error::NetworkError(format!("Failed to fetch atomical info: {}", e)))?;
            
        let unisat_response: UnisatResponse = response.json().await
            .map_err(|e| Error::DeserializationError(format!("Failed to deserialize response: {}", e)))?;

        if unisat_response.code != 0 {
            return Err(Error::AtomicalNotFound(format!("Atomical {} not found: {}", atomical_id, unisat_response.msg)));
        }

        // 将 UniSat 的响应格式转换为我们的 AtomicalInfo 格式
        let mut atomical_info: AtomicalInfo = serde_json::from_value(unisat_response.data)
            .map_err(|e| Error::DeserializationError(format!("Failed to convert response format: {}", e)))?;

        // 确保 location 字段格式正确
        if let Some(location) = atomical_info.location_info.first_mut() {
            if !location.location.contains(':') {
                location.location = format!("{}:{}", location.txid, location.index);
            }
        }

        Ok(atomical_info)
    }
}

#[derive(Debug, Deserialize)]
struct UnisatResponse {
    code: i32,
    msg: String,
    data: Value,
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
