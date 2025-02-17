use crate::errors::{Error, Result};
use crate::wallet::{WalletProvider, Utxo};
use async_trait::async_trait;
use bitcoin::{Transaction, TxOut, Network, PublicKey, Amount, OutPoint, Psbt};
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

    async fn get_utxos(&self) -> Result<Vec<Utxo>> {
        // 调用钱包的 getUtxos 方法
        let result = self.call_wallet_method("getUtxos", &[])?;
        
        // 等待 Promise 完成
        let utxos_js = JsFuture::from(result.unchecked_into::<Promise>()).await
            .map_err(|e| Error::WalletError(format!("Failed to get UTXOs: {:?}", e)))?;
            
        let utxos_array: Array = utxos_js.unchecked_into();
        let mut utxos = Vec::new();
        
        for i in 0..utxos_array.length() {
            let utxo_js = utxos_array.get(i);
            let utxo_obj: Object = utxo_js.unchecked_into();
            
            // 获取 UTXO 的各个字段
            let txid = Reflect::get(&utxo_obj, &JsValue::from_str("txid"))?;
            let vout = Reflect::get(&utxo_obj, &JsValue::from_str("vout"))?;
            let value = Reflect::get(&utxo_obj, &JsValue::from_str("value"))?;
            let script_pubkey = Reflect::get(&utxo_obj, &JsValue::from_str("scriptPubKey"))?;
            let height = Reflect::get(&utxo_obj, &JsValue::from_str("height"))?;
            
            // 转换数据类型
            let txid: String = from_value(txid)?;
            let vout: u32 = from_value(vout)?;
            let value: u64 = from_value(value)?;
            let script_pubkey: String = from_value(script_pubkey)?;
            let height: Option<u32> = from_value(height).ok();
            
            // 创建 OutPoint
            let outpoint = OutPoint::new(
                bitcoin::Txid::from_str(&txid)
                    .map_err(|e| Error::WalletError(format!("Invalid txid: {}", e)))?,
                vout,
            );
            
            // 创建 TxOut
            let txout = TxOut {
                value: Amount::from_sat(value),
                script_pubkey: bitcoin::ScriptBuf::from_hex(&script_pubkey)
                    .map_err(|e| Error::WalletError(format!("Invalid script: {}", e)))?,
            };
            
            utxos.push(Utxo {
                outpoint,
                txout,
                height,
            });
        }
        
        Ok(utxos)
    }

    async fn get_balance(&self) -> Result<Amount> {
        // 调用钱包的 getBalance 方法
        let result = self.call_wallet_method("getBalance", &[])?;
        
        // 等待 Promise 完成
        let balance_js = JsFuture::from(result.unchecked_into::<Promise>()).await
            .map_err(|e| Error::WalletError(format!("Failed to get balance: {:?}", e)))?;
            
        // 转换为 satoshis
        let balance: u64 = from_value(balance_js)?;
        Ok(Amount::from_sat(balance))
    }

    async fn get_network_fee_rate(&self) -> Result<f64> {
        // 调用钱包的 getFeeRate 方法
        let result = self.call_wallet_method("getFeeRate", &[])?;
        
        // 等待 Promise 完成
        let fee_rate_js = JsFuture::from(result.unchecked_into::<Promise>()).await
            .map_err(|e| Error::WalletError(format!("Failed to get fee rate: {:?}", e)))?;
            
        // 转换为 f64
        let fee_rate: f64 = from_value(fee_rate_js)?;
        Ok(fee_rate)
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
