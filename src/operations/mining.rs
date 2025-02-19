use crate::errors::{Error, Result};
use crate::types::mint::BitworkInfo;
use crate::types::wasm::{WasmTransaction, WasmBitworkInfo};
use bitcoin::Transaction;
use bitcoin::transaction::Version;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use js_sys::{Promise, Object, Reflect};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Worker, MessageEvent, Event};
use std::cell::RefCell;
use std::rc::Rc;
use serde_wasm_bindgen;
use hex;
use sha2::{Sha256, Digest};
use serde_json::json;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct MiningOptions {
    pub num_workers: u32,
    pub batch_size: u32,
}

#[wasm_bindgen]
impl MiningOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            num_workers: 4,
            batch_size: 100000,  // 增加默认批处理大小
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MiningResult {
    pub success: bool,
    pub nonce: Option<u32>,
    #[serde(skip)]
    pub tx_hex: Option<String>,
}

impl MiningResult {
    pub fn with_transaction(success: bool, nonce: Option<u32>, tx: Option<&Transaction>) -> Self {
        Self {
            success,
            nonce,
            tx_hex: tx.map(|t| hex::encode(bitcoin::consensus::serialize(t))),
        }
    }

    pub fn get_transaction(&self) -> Option<Transaction> {
        self.tx_hex.as_ref().and_then(|hex| {
            hex::decode(hex)
                .ok()
                .and_then(|bytes| bitcoin::consensus::deserialize(&bytes).ok())
        })
    }
}

// 在Rust中进行挖矿计算的函数
#[wasm_bindgen]
pub fn mine_nonce_range(
    tx_wrapper: &WasmTransaction,
    start_nonce: u32,
    end_nonce: u32,
    bitwork: &str,
) -> Option<u32> {
    log!("开始挖矿 - 范围: {} 到 {}, bitwork: {}", start_nonce, end_nonce, bitwork);
    
    let progress_interval = 10000; // 每10000个nonce报告一次进度
    let mut nonces_tried = 0;
    
    for nonce in start_nonce..=end_nonce {
        // 定期报告进度
        if nonce % progress_interval == 0 {
            nonces_tried += progress_interval;
            let progress = ((nonce - start_nonce) as f64 / (end_nonce - start_nonce) as f64 * 100.0) as u32;
            log!("挖矿进度: {}%, 已尝试 {} 个nonce值", progress, nonces_tried);
        }
        
        // 检查当前nonce是否产生有效的交易ID
        if let Some(valid_nonce) = check_nonce(tx_wrapper, nonce, bitwork) {
            log!("找到有效nonce: {}, 满足bitwork要求: {}", valid_nonce, bitwork);
            return Some(valid_nonce);
        }
    }
    
    log!("完成范围 {} 到 {} 的搜索，未找到满足条件的nonce", start_nonce, end_nonce);
    None
}

fn check_nonce(tx: &WasmTransaction, nonce: u32, bitwork: &str) -> Option<u32> {
    let nonce_hex = format!("{:08x}", nonce);
    let data_with_nonce = format!("{}{}", tx.to_hex(), nonce_hex);
    
    let mut hasher = Sha256::new();
    hasher.update(hex::decode(&data_with_nonce).unwrap());
    let result = hasher.finalize();
    let txid = hex::encode(result);
    
    // 首先检查基本的bitwork前缀
    if has_valid_bitwork(&txid, bitwork, None) {
        Some(nonce)
    } else {
        None
    }
}

fn has_valid_bitwork(txid: &str, bitwork: &str, bitworkx: Option<u32>) -> bool {
    if txid.starts_with(bitwork) {
        if let Some(x) = bitworkx {
            if let Some(next_char) = txid.chars().nth(bitwork.len()) {
                let char_value = match next_char {
                    '0'..='9' => next_char as u32 - '0' as u32,
                    'a'..='f' => next_char as u32 - 'a' as u32 + 10,
                    _ => return false,
                };
                return char_value >= x;
            }
        }
        return true;
    }
    false
}

// 创建Web Worker实例
fn create_worker(
    tx_wrapper: &WasmTransaction,
    start_nonce: u32,
    end_nonce: u32,
    bitwork_info: &BitworkInfo,
    shared_result: Rc<RefCell<MiningResult>>,
    workers: Rc<RefCell<Vec<Worker>>>,
) -> Result<Worker> {
    log!("Creating worker for nonce range: {} to {}", start_nonce, end_nonce);
    
    // 获取当前脚本的位置
    let location = web_sys::window()
        .ok_or_else(|| Error::WorkerError("Failed to get window".into()))?
        .location();
    
    let origin = location.origin()
        .map_err(|_| Error::WorkerError("Failed to get origin".into()))?;
    
    // 构建完整的 Worker 脚本路径
    let worker_url = format!("{}/worker_entry.js", origin);
    log!("Worker script URL: {}", worker_url);
    
    // 创建Worker
    let worker = Worker::new(&worker_url)
        .map_err(|e| Error::WorkerError(format!("Failed to create worker: {:?}", e)))?;
    
    // 监听 worker 消息
    let worker_clone = worker.clone();
    let workers_ref = workers.clone();
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(data) = e.data().dyn_into::<Object>() {
            if let Ok(msg_type) = Reflect::get(&data, &"type".into()) {
                match msg_type.as_string().as_deref() {
                    Some("success") => {
                        if let (Ok(success), Ok(nonce)) = (
                            Reflect::get(&data, &"success".into()),
                            Reflect::get(&data, &"nonce".into()),
                        ) {
                            if success.as_bool().unwrap_or(false) {
                                let nonce_value = nonce.as_f64().map(|n| n as u32);
                                log!("Worker received result: success=true, nonce={:?}", nonce_value);
                                
                                // 更新共享结果
                                let mut result = shared_result.borrow_mut();
                                result.success = true;
                                result.nonce = nonce_value;
                                
                                // 向所有 worker 发送停止信号
                                for w in workers_ref.borrow().iter() {
                                    let stop_msg = Object::new();
                                    Reflect::set(&stop_msg, &"type".into(), &"stop".into()).unwrap();
                                    let _ = w.post_message(&stop_msg);
                                }
                            }
                        }
                    }
                    Some("progress") => {
                        // 处理进度更新
                        if let Ok(progress) = Reflect::get(&data, &"progress".into()) {
                            log!("Mining progress update: {:?}", progress);
                        }
                    }
                    Some("exhausted") => {
                        log!("Worker exhausted its nonce range");
                    }
                    Some("error") => {
                        log!("Worker encountered an error");
                    }
                    _ => {}
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    
    worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
    
    // 设置错误处理器
    let worker_clone = worker.clone();
    let onerror_callback = Closure::wrap(Box::new(move |e: Event| {
        log!("Worker error occurred, terminating worker");
        worker_clone.terminate();
    }) as Box<dyn FnMut(Event)>);

    worker.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    
    // 准备发送到 Worker 的数据
    let mut task_obj = Object::new();
    
    // 使用 serde_wasm_bindgen 序列化数据
    let tx_value = serde_wasm_bindgen::to_value(tx_wrapper)
        .map_err(|e| Error::SerializationError(format!("Failed to serialize tx_wrapper: {}", e)))?;
    
    Reflect::set(&task_obj, &"tx_wrapper".into(), &tx_value)?;
    Reflect::set(&task_obj, &"start_nonce".into(), &JsValue::from(start_nonce))?;
    Reflect::set(&task_obj, &"end_nonce".into(), &JsValue::from(end_nonce))?;
    Reflect::set(&task_obj, &"bitwork".into(), &JsValue::from(&bitwork_info.prefix))?;
    
    // 发送任务到Worker
    worker.post_message(&task_obj)
        .map_err(|e| Error::WorkerError(format!("Failed to post message: {:?}", e)))?;
    
    log!("Worker initialized successfully");
    Ok(worker)
}

#[wasm_bindgen]
pub async fn mine_transaction(
    tx_wrapper: WasmTransaction,
    bitwork_wrapper: WasmBitworkInfo,
    options: MiningOptions,
) -> Result<JsValue> {
    log!("Starting mining with {} workers", options.num_workers);
    
    let tx = tx_wrapper.to_transaction()
        .ok_or_else(|| Error::DeserializationError("Failed to deserialize transaction".into()))?;
    let bitwork = bitwork_wrapper.to_bitwork_info();
    
    let shared_result = Rc::new(RefCell::new(MiningResult {
        success: false,
        nonce: None,
        tx_hex: None,
    }));
    
    let workers = Rc::new(RefCell::new(Vec::new()));
    
    // 定义最大 nonce 值
    const MAX_SEQUENCE: u32 = 0xffffffff;
    let range_per_worker = MAX_SEQUENCE / options.num_workers;
    
    // 创建多个Worker进行并行挖矿
    for i in 0..options.num_workers {
        let start_nonce = i * range_per_worker;
        let end_nonce = if i == options.num_workers - 1 {
            MAX_SEQUENCE  // 最后一个 worker 处理到最大值
        } else {
            start_nonce + range_per_worker - 1
        };
        
        log!("Creating worker {} with range: {} to {}", i, start_nonce, end_nonce);
        
        let worker = create_worker(
            &tx_wrapper,
            start_nonce,
            end_nonce,
            &bitwork,
            shared_result.clone(),
            workers.clone(),
        )?;
        
        workers.borrow_mut().push(worker);
    }
    
    // 等待挖矿完成或超时
    let workers_clone = workers.clone();
    let shared_result_clone = shared_result.clone();
    
    let promise = Promise::new(&mut |resolve, _reject| {
        let timeout_ms = 300000; // 5 minutes
        let window = web_sys::window().unwrap();
        
        // 创建检查结果的间隔
        let check_interval = 100; // 每100ms检查一次
        
        // 创建用于检查结果的闭包
        let check_callback = {
            let workers_inner = workers_clone.clone();
            let shared_result_inner = shared_result_clone.clone();
            let window = window.clone();
            let resolve = resolve.clone();
            
            Closure::wrap(Box::new(move || {
                let result = shared_result_inner.borrow();
                if result.success {
                    // 找到有效 nonce，停止所有 worker
                    for worker in workers_inner.borrow().iter() {
                        let stop_msg = Object::new();
                        Reflect::set(&stop_msg, &"type".into(), &"stop".into()).unwrap();
                        let _ = worker.post_message(&stop_msg);
                        worker.terminate();
                    }
                    
                    // 解析结果
                    resolve.call1(&JsValue::NULL, &serde_wasm_bindgen::to_value(&*result).unwrap()).unwrap();
                }
            }) as Box<dyn FnMut()>)
        };
        
        // 设置定时器
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            check_callback.as_ref().unchecked_ref(),
            check_interval,
        ).unwrap();
        
        // 设置超时
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            &resolve,
            timeout_ms,
        ).unwrap();
        
        // 保持closure存活
        check_callback.forget();
    });
    
    // 等待结果
    JsFuture::from(promise).await
        .map_err(|e| Error::AsyncError(format!("Promise error: {:?}", e)))?;
    
    // 获取最终结果
    let final_result = shared_result.borrow().clone();
    
    // 确保所有 worker 都已停止
    for worker in workers.borrow().iter() {
        worker.terminate();
    }
    
    // 返回结果
    serde_wasm_bindgen::to_value(&final_result)
        .map_err(|e| Error::SerializationError(format!("Failed to serialize result: {}", e)))
}

pub fn verify_bitwork(tx: &Transaction, bitwork: &BitworkInfo) -> bool {
    let tx_hash = tx.txid().to_string();
    tx_hash.starts_with(&bitwork.prefix)
}
