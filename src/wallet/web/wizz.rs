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
use web_sys::{window, console};
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct WizzProvider {
    wallet: Object,
    account: Option<String>,
}

#[wasm_bindgen]
impl WizzProvider {
    #[wasm_bindgen(constructor)]
    pub fn try_new() -> std::result::Result<WizzProvider, JsValue> {
        let window = window().ok_or_else(|| JsValue::from_str("Window not available"))?;
        let wizz = Reflect::get(&window, &JsValue::from_str("wizz"))
            .map_err(|_| JsValue::from_str("Wizz wallet not found"))?;
        
        Ok(WizzProvider {
            wallet: wizz.unchecked_into(),
            account: None,
        })
    }
}

#[async_trait(?Send)]
impl WalletProvider for WizzProvider {
    async fn get_network(&self) -> Result<Network> {
        // 检查 accounts 是否存在
        let accounts = self.call_wallet_method("accounts", &[])?;
        let accounts_array: Array = accounts.unchecked_into();
        if accounts_array.length() == 0 {
            // 如果没有账户，尝试连接
            self.call_wallet_method("requestAccounts", &[])?;
        }

        // 默认使用 testnet
        Ok(Network::Testnet)
    }

    async fn get_public_key(&self) -> Result<PublicKey> {
        // 尝试从 accounts 获取地址
        let accounts = self.call_wallet_method("accounts", &[])?;
        let accounts_array: Array = accounts.unchecked_into();
        
        if accounts_array.length() == 0 {
            return Err(Error::WalletError("No accounts available".to_string()));
        }

        let account = accounts_array.get(0);
        let account_str: String = from_value(account)?;

        // 将地址转换为公钥（这里需要实际的转换逻辑）
        Err(Error::WalletError("Public key not available".to_string()))
    }

    async fn get_address(&self) -> Result<String> {
        // 先检查钱包是否已连接
        let is_connected = Reflect::get(&self.wallet, &JsValue::from_str("isConnected"))
            .map_err(|_| Error::WalletError("Failed to check wallet connection".to_string()))?;
            
        if !is_connected.is_truthy() {
            // 如果未连接，先尝试连接
            let connect_result = self.call_wallet_method("connect", &[])?;
            if !connect_result.is_undefined() {
                // 等待连接完成
                if let Ok(promise) = JsFuture::from(connect_result.unchecked_into::<Promise>()).await {
                    log!("Wallet connected successfully");
                }
            }
        }

        // 请求账户
        let request_result = self.call_wallet_method("requestAccounts", &[])?;
        if !request_result.is_undefined() {
            // 等待请求完成
            if let Ok(accounts) = JsFuture::from(request_result.unchecked_into::<Promise>()).await {
                if js_sys::Array::is_array(&accounts) {
                    let accounts_array: js_sys::Array = accounts.unchecked_into();
                    if accounts_array.length() > 0 {
                        let account = accounts_array.get(0);
                        return from_value(account)
                            .map_err(|e| Error::WalletError(format!("Failed to parse address: {}", e)));
                    }
                }
            }
        }

        // 如果上述方法都失败，尝试直接获取 selectedAddress
        let selected_address = Reflect::get(&self.wallet, &JsValue::from_str("selectedAddress"))
            .map_err(|_| Error::WalletError("Failed to get selected address".to_string()))?;
            
        if !selected_address.is_undefined() {
            return from_value(selected_address)
                .map_err(|e| Error::WalletError(format!("Failed to parse address: {}", e)));
        }

        Err(Error::WalletError("No accounts available".to_string()))
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
        let result = self.call_wallet_method("sendTransaction", &[to_value(&tx_hex)?])?;
        
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

impl WizzProvider {
    fn call_wallet_method(&self, method: &str, args: &[JsValue]) -> Result<JsValue> {
        let method_fn = Reflect::get(&self.wallet, &JsValue::from_str(method))
            .map_err(|_| Error::WalletError(format!("Method {} not found", method)))?;
        
        if !method_fn.is_function() {
            // 如果方法不是函数，尝试作为属性获取
            return Ok(method_fn);
        }

        let method_fn: Function = method_fn.unchecked_into();
        let args_array = Array::new();
        for arg in args {
            args_array.push(arg);
        }
        
        let this = JsValue::from(self.wallet.clone());
        method_fn.apply(&this, &args_array)
            .map_err(|e| Error::WalletError(format!("Failed to call {}: {:?}", method, e)))
    }
}
