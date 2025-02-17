use crate::errors::{Error, Result};
use crate::wallet::{WalletProvider, Utxo};
use async_trait::async_trait;
use bitcoin::{Transaction, TxOut, Network, PublicKey, Amount, OutPoint, Psbt, Address};
use wasm_bindgen::prelude::*;
use js_sys::{Function, Object, Promise, Reflect, Array};
use serde_wasm_bindgen::{to_value, from_value};
use std::str::FromStr;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use web_sys::{window, console};
use wasm_bindgen_futures::JsFuture;
use serde::{Deserialize, Serialize};
use reqwest;
use serde_json::Value;
use bitcoin::hashes::{sha256, Hash};

#[derive(Debug, Deserialize)]
struct UtxoResponse {
    success: bool,
    response: ResponseData,
    cache: bool,
}

#[derive(Debug, Deserialize)]
struct ResponseData {
    atomicals: Value,
    global: GlobalInfo,
    utxos: Vec<UtxoItem>,
}

#[derive(Debug, Deserialize)]
struct GlobalInfo {
    atomical_count: u32,
    height: u32,
    network: String,
    server_time: String,
    #[serde(flatten)]
    other: Value,
}

#[derive(Debug, Deserialize)]
struct UtxoItem {
    height: u32,
    txid: String,
    value: u64,
    vout: u32,
    index: u32,
    #[serde(default)]
    atomicals: Value,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct WizzProvider {
    wallet: Object,
    account: Option<String>,
}

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
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
        log!("Getting wallet address");
        
        // 先检查钱包是否已连接
        let is_connected = Reflect::get(&self.wallet, &JsValue::from_str("isConnected"))
            .map_err(|e| {
                log!("Failed to check wallet connection: {:?}", e);
                Error::WalletError("Failed to check wallet connection".to_string())
            })?;
            
        log!("Wallet connection status: {}", is_connected.is_truthy());
            
        if !is_connected.is_truthy() {
            log!("Wallet not connected, attempting to connect");
            // 如果未连接，先尝试连接
            let connect_result = self.call_wallet_method("connect", &[])?;
            
            log!("Connect method called, result type: {:?}", connect_result.js_typeof());
            
            if !connect_result.is_undefined() {
                log!("Waiting for connect promise to resolve");
                // 等待连接完成
                if let Ok(promise) = JsFuture::from(connect_result.unchecked_into::<Promise>()).await {
                    log!("Wallet connected successfully");
                }
            }
        }

        log!("Requesting accounts");
        // 请求账户
        let request_result = self.call_wallet_method("requestAccounts", &[])?;
        
        log!("Request accounts result type: {:?}", request_result.js_typeof());
        
        if !request_result.is_undefined() {
            log!("Waiting for requestAccounts promise to resolve");
            // 等待请求完成
            if let Ok(accounts) = JsFuture::from(request_result.unchecked_into::<Promise>()).await {
                log!("Accounts received, checking array");
                if js_sys::Array::is_array(&accounts) {
                    let accounts_array: js_sys::Array = accounts.unchecked_into();
                    log!("Found {} accounts", accounts_array.length());
                    if accounts_array.length() > 0 {
                        let account = accounts_array.get(0);
                        return from_value(account)
                            .map_err(|e| {
                                log!("Failed to parse account: {:?}", e);
                                Error::WalletError("Failed to parse account".to_string())
                            });
                    }
                }
            }
        }
        
        log!("No accounts found");
        Err(Error::WalletError("No accounts available".to_string()))
    }

    async fn get_utxos(&self) -> Result<Vec<Utxo>> {
        log!("Getting UTXOs");
        
        // 先确保钱包已连接并获取地址
        let address = self.get_address().await?;
        log!("Got address: {}", address);
        
        // 获取 scripthash
        let addr = Address::from_str(&address)
            .map_err(|e| Error::AddressError(format!("Invalid address: {}", e)))?
            .require_network(Network::Testnet)
            .map_err(|e| Error::AddressError(format!("Invalid network: {}", e)))?;
            
        let script_pubkey = addr.script_pubkey();
        
        // 计算 scripthash
        let script_bytes = script_pubkey.as_bytes();
        let hash = sha256::Hash::hash(script_bytes);
        
        // 反转字节序并转换为十六进制字符串
        let scripthash = hash.to_byte_array()
            .iter()
            .rev()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        
        log!("Calculated scripthash: {}", scripthash);
        
        // 构建 API URL
        let api_url = format!(
            "https://eptestnet4.wizz.cash/proxy/blockchain.atomicals.listscripthash?params=[\"{}\",true]",
            scripthash
        );
        
        log!("Fetching UTXOs from API: {}", api_url);
        
        // 发起 HTTP 请求
        let response = reqwest::get(&api_url).await
            .map_err(|e| Error::NetworkError(format!("Failed to fetch UTXOs: {}", e)))?;
            
        let utxo_response: UtxoResponse = response.json().await
            .map_err(|e| Error::DeserializationError(format!("Failed to parse UTXO response: {}", e)))?;
            
        if !utxo_response.success {
            return Err(Error::NetworkError("UTXO API request failed".into()));
        }
        
        let total_utxos = utxo_response.response.utxos.len();
        
        // 过滤出 atomicals 为空的 UTXO
        let filtered_utxos: Vec<UtxoItem> = utxo_response.response.utxos.into_iter()
            .filter(|utxo| utxo.atomicals.as_object().map_or(true, |obj| obj.is_empty()))
            .collect();
        
        let available_utxos = filtered_utxos.len();
        log!("Found {} total UTXOs, {} available after filtering", 
            total_utxos,
            available_utxos
        );
        
        // 转换为 Utxo 结构
        let utxos = filtered_utxos.into_iter()
            .map(|item| {
                let outpoint = OutPoint::new(
                    bitcoin::Txid::from_str(&item.txid)
                        .map_err(|e| Error::TransactionError(format!("Invalid txid: {}", e)))?,
                    item.vout,
                );
                
                let txout = TxOut {
                    value: Amount::from_sat(item.value),
                    script_pubkey: script_pubkey.clone(),
                };
                
                Ok(Utxo {
                    outpoint,
                    txout,
                    height: Some(item.height),
                })
            })
            .collect::<Result<Vec<_>>>()?;
            
        log!("Successfully parsed {} available UTXOs", utxos.len());
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
        log!("Calling wallet method: {}", method);
        
        let method_fn = Reflect::get(&self.wallet, &JsValue::from_str(method))
            .map_err(|_| Error::WalletError(format!("Method {} not found", method)))?;
        
        log!("Method found, is_function: {}", method_fn.is_function());
        
        if !method_fn.is_function() {
            log!("Method is not a function, returning as property");
            return Ok(method_fn);
        }

        let method_fn: Function = method_fn.unchecked_into();
        let args_array = Array::new();
        for arg in args {
            args_array.push(arg);
        }
        
        log!("Applying method with {} arguments", args_array.length());
        
        let this = JsValue::from(self.wallet.clone());
        let result = method_fn.apply(&this, &args_array)
            .map_err(|e| {
                log!("Method call failed: {:?}", e);
                Error::WalletError(format!("Failed to call {}: {:?}", method, e))
            })?;
            
        log!("Method call succeeded, checking if result is Promise");
        log!("Result type: {:?}", result.js_typeof());
            
        // 检查结果是否是 Promise 实例
        let is_promise = match js_sys::Reflect::get(&result, &JsValue::from_str("then")) {
            Ok(then) => {
                let is_fn = then.is_function();
                log!("Found 'then' method, is_function: {}", is_fn);
                is_fn
            },
            Err(e) => {
                log!("Error checking 'then' method: {:?}", e);
                false
            }
        };
            
        if is_promise {
            log!("Returning Promise");
            return Ok(result);
        }
        
        log!("Returning non-Promise result");
        Ok(result)
    }
}
