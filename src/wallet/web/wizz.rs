use crate::errors::{Error, Result};
use crate::wallet::{WalletProvider, Utxo};
use crate::types::atomicals::{AtomicalInfo, AtomicalResponse, AtomicalResponseData};
use async_trait::async_trait;
use bitcoin::{Transaction, TxOut, Network, PublicKey, Amount, OutPoint, Psbt, Address, Txid};
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
use hex;


#[derive(Debug, Deserialize)]
struct UtxoResponse {
    success: bool,
    response: ResponseData,
    #[serde(default)]
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

#[derive(Debug, Deserialize)]
struct MempoolBlock {
    #[serde(rename = "blockSize")]
    block_size: f64,
    #[serde(rename = "blockVSize")]
    block_vsize: f64,
    #[serde(rename = "nTx")]
    n_tx: u32,
    #[serde(rename = "totalFees")]
    total_fees: f64,
    #[serde(rename = "medianFee")]
    median_fee: f64,
    #[serde(rename = "feeRange")]
    fee_range: Vec<f64>,
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
        log!("Requesting public key");
        let result = self.call_wallet_method("getPublicKey", &[])?;
        
        log!("Public key result type: {:?}", result.js_typeof());
        
        if result.is_object() {
            log!("Waiting for getPublicKey promise to resolve");
            
            if let Ok(pubkey_value) = JsFuture::from(result.unchecked_into::<Promise>()).await {
                log!("Public key received");
                let pubkey_hex = pubkey_value.as_string()
                    .ok_or_else(|| Error::WalletError("Invalid public key format".to_string()))?;
                
                // 移除可能的 "0x" 前缀
                let pubkey_hex = pubkey_hex.trim_start_matches("0x");
                
                // 将十六进制字符串转换为 PublicKey
                return PublicKey::from_str(&pubkey_hex)
                    .map_err(|e| Error::WalletError(format!("Failed to parse public key: {}", e)));
            }
        }
        
        Err(Error::WalletError("Failed to get public key".to_string()))
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
            
        let response_text = response.text().await
            .map_err(|e| Error::NetworkError(format!("Failed to get response text: {}", e)))?;
            
        log!("API Response: {}", response_text);
        
        let utxo_response: UtxoResponse = serde_json::from_str(&response_text)
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
        log!("Getting fee rate from mempool.space API");
        
        let api_url = "https://mempool.space/testnet4/api/v1/fees/mempool-blocks";
        
        // 发起 HTTP 请求
        let response = reqwest::get(api_url).await
            .map_err(|e| Error::NetworkError(format!("Failed to fetch fee rate: {}", e)))?;
            
        let mempool_blocks: Vec<MempoolBlock> = response.json().await
            .map_err(|e| Error::DeserializationError(format!("Failed to parse fee rate response: {}", e)))?;
            
        // 获取第一个区块的费率
        let fee_rate = mempool_blocks.first()
            .map(|block| {
                if block.median_fee == 0.0 {
                    1.0
                } else {
                    block.median_fee
                }
            })
            .unwrap_or(1.0);
            
        log!("Got fee rate: {} sat/vB", fee_rate);
        
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
        // 将交易转换为十六进制
        let tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
        
        // 直接调用 pushPsbt 方法
        let promise = self.call_wallet_method("pushTx", &[to_value(&tx_hex)?])?;
        let promise = js_sys::Promise::from(promise);
        let result = wasm_bindgen_futures::JsFuture::from(promise).await
            .map_err(|e| Error::WalletError(format!("Failed to broadcast transaction: {:?}", e)))?;
        
        // 返回交易ID
        from_value(result)
            .map_err(|e| Error::WalletError(format!("Failed to parse txid: {}", e)))
    }

    async fn sign_psbt(&self, psbt: Psbt) -> Result<Psbt> {
        let psbt_bytes = psbt.serialize();
        let psbt_hex = hex::encode(psbt_bytes);
        
        log!("Sending PSBT to wallet for signing: {}", &psbt_hex);
        
        // 调用钱包方法并等待 Promise 完成
        let promise = self.call_wallet_method("signPsbt", &[to_value(&psbt_hex)?])?;
        let promise = js_sys::Promise::from(promise);
        let result = wasm_bindgen_futures::JsFuture::from(promise).await
            .map_err(|e| Error::WalletError(format!("Failed to await signPsbt promise: {:?}", e)))?;
        
        // 从 JsValue 转换为字符串
        let signed_psbt_hex: String = from_value(result)
            .map_err(|e| Error::WalletError(format!("Failed to parse signed PSBT result: {}", e)))?;
            
        log!("Received signed PSBT from wallet: {}", &signed_psbt_hex);
        
        // 解码十六进制字符串
        let signed_psbt_bytes = hex::decode(&signed_psbt_hex)
            .map_err(|e| Error::WalletError(format!("Failed to decode hex PSBT: {}", e)))?;
        
        let signed_psbt = Psbt::deserialize(&signed_psbt_bytes)
            .map_err(|e| Error::WalletError(format!("Failed to parse signed PSBT: {}", e)))?;
            
        // 验证PSBT是否已经完全签名
        if signed_psbt.inputs.iter().any(|input| input.final_script_sig.is_none() && input.final_script_witness.is_none()) {
            return Err(Error::WalletError("PSBT signing incomplete".into()));
        }
        
        Ok(signed_psbt)
    }

    async fn get_atomical_by_id(&self, atomical_id: &str) -> Result<AtomicalInfo> {
        let url = format!(
            "https://eptestnet4.wizz.cash/proxy/blockchain.atomicals.get_state?params=[\"{}\"]&_={}",
            atomical_id,
            js_sys::Date::now() as u64
        );

        let response = reqwest::get(&url).await
            .map_err(|e| Error::NetworkError(format!("Failed to fetch atomical info: {}", e)))?;
            
        let atomical_response: AtomicalResponse = response.json().await
            .map_err(|e| Error::DeserializationError(format!("Failed to deserialize response: {}", e)))?;

        if !atomical_response.success {
            return Err(Error::AtomicalNotFound(format!("Atomical {} not found", atomical_id)));
        }

        let mut result = atomical_response.response.result;
        
        // 确保 location 字段格式正确
        for location in result.location_info.iter_mut() {
            if !location.location.contains(':') {
                // 使用 txid 和 index 构建正确的 location 格式
                location.location = format!("{}:{}", location.txid, location.index);
                log!("Fixed location format: {} -> {}", location.txid, location.location);
            }
        }

        Ok(result)
    }
}

impl WizzProvider {
    fn call_wallet_method(&self, method: &str, args: &[JsValue]) -> Result<JsValue> {
        // log!("Calling wallet method: {}", method);
        
        let method_fn = Reflect::get(&self.wallet, &JsValue::from_str(method))
            .map_err(|_| Error::WalletError(format!("Method {} not found", method)))?;
        
        // log!("Method found, is_function: {}", method_fn.is_function());
        
        if !method_fn.is_function() {
            log!("Method is not a function, returning as property");
            return Ok(method_fn);
        }

        let method_fn: Function = method_fn.unchecked_into();
        let args_array = Array::new();
        for arg in args {
            args_array.push(arg);
        }
        
        // log!("Applying method with {} arguments", args_array.length());
        
        let this = JsValue::from(self.wallet.clone());
        let result = method_fn.apply(&this, &args_array)
            .map_err(|e| {
                log!("Method call failed: {:?}", e);
                Error::WalletError(format!("Failed to call {}: {:?}", method, e))
            })?;
            
        // log!("Method call succeeded, checking if result is Promise");
        // log!("Result type: {:?}", result.js_typeof());
            
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
