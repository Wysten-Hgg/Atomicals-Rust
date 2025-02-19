use crate::errors::{Error, Result};
use crate::types::mint::BitworkInfo;
use crate::types::wasm::{WasmTransaction, WasmBitworkInfo};
use bitcoin::Transaction;
use bitcoin::transaction::Version;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use js_sys::{Promise, Array, Object, Reflect};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Worker, MessageEvent, Event};
use std::cell::RefCell;
use std::rc::Rc;
use serde_wasm_bindgen;
use hex;

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
            batch_size: 1000,
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
    if let Some(mut current_tx) = tx_wrapper.to_transaction() {
        for nonce in start_nonce..end_nonce {
            current_tx.version = Version(nonce as i32);
            let tx_hash = current_tx.txid().to_string();
            if tx_hash.starts_with(bitwork) {
                return Some(nonce);
            }
        }
    }
    None
}

// 创建Web Worker实例
fn create_worker(
    tx_wrapper: &WasmTransaction,
    start_nonce: u32,
    end_nonce: u32,
    bitwork_info: &BitworkInfo,
    shared_result: Rc<RefCell<MiningResult>>,
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
    
    // 设置消息处理器
    let worker_result = shared_result.clone();
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        match serde_wasm_bindgen::from_value::<MiningResult>(e.data()) {
            Ok(result) => {
                log!("Worker received result: success={}, nonce={:?}", result.success, result.nonce);
                if result.success {
                    let mut current_result = worker_result.borrow_mut();
                    *current_result = result;
                }
            }
            Err(e) => {
                log!("Worker message deserialization error: {}", e);
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    // 设置错误处理器
    let worker_clone = worker.clone();
    let onerror_callback = Closure::wrap(Box::new(move |e: Event| {
        log!("Worker error occurred, terminating worker");
        worker_clone.terminate();
    }) as Box<dyn FnMut(Event)>);

    worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    worker.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
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
    
    let mut workers = Vec::new();
    
    // 创建多个Worker进行并行挖矿
    for i in 0..options.num_workers {
        let start_nonce = i * options.batch_size;
        let end_nonce = start_nonce + options.batch_size;
        
        let worker = create_worker(
            &tx_wrapper,
            start_nonce,
            end_nonce,
            &bitwork,
            shared_result.clone(),
        )?;
        
        workers.push(worker);
    }
    
    // 等待结果
    let promise = Promise::new(&mut |resolve, _reject| {
        let timeout_ms = 300000; // 5 minutes
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                timeout_ms,
            )
            .unwrap();
    });
    
    JsFuture::from(promise).await
        .map_err(|e| Error::AsyncError(format!("Promise error: {:?}", e)))?;
    
    // 获取最终结果
    let final_result = shared_result.borrow().clone();
    
    // 停止所有Worker
    for worker in workers {
        worker.terminate();
    }
    
    // 验证结果
    let result = if final_result.success {
        if let Some(nonce) = final_result.nonce {
            let mut final_tx = tx.clone();
            final_tx.version = Version(nonce as i32);
            
            if verify_bitwork(&final_tx, &bitwork) {
                MiningResult::with_transaction(true, Some(nonce), Some(&final_tx))
            } else {
                log!("Mining result verification failed");
                MiningResult {
                    success: false,
                    nonce: None,
                    tx_hex: None,
                }
            }
        } else {
            final_result
        }
    } else {
        final_result
    };
    
    // 转换结果为JsValue
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| Error::SerializationError(e.to_string()))
}

pub fn verify_bitwork(tx: &Transaction, bitwork: &BitworkInfo) -> bool {
    let tx_hash = tx.txid().to_string();
    tx_hash.starts_with(&bitwork.prefix)
}
