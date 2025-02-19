use wasm_bindgen::prelude::*;
use web_sys::DedicatedWorkerGlobalScope;
use serde::{Serialize, Deserialize};
use bitcoin::Transaction;

#[wasm_bindgen]
pub fn init_worker() {
    let worker_scope = js_sys::global()
        .dyn_into::<DedicatedWorkerGlobalScope>()
        .unwrap();
        
    let callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Ok(task) = e.data().into_serde::<MiningTask>() {
            // 解析交易数据
            let tx = bitcoin::consensus::encode::deserialize_hex(&task.tx_hex)
                .expect("Failed to deserialize transaction");
                
            // 调用Rust的挖矿函数
            if let Some(nonce) = mine_nonce_range(&tx, task.start_nonce, task.end_nonce, &task.bitwork) {
                let result = MiningResult {
                    success: true,
                    nonce: Some(nonce),
                    tx: None,
                };
                
                let result_js = JsValue::from_serde(&result)
                    .expect("Failed to serialize result");
                    
                worker_scope.post_message(&result_js)
                    .expect("Failed to post message");
            }
        }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    
    worker_scope.set_onmessage(Some(callback.as_ref().unchecked_ref()));
    callback.forget();
}

#[derive(Serialize, Deserialize)]
struct MiningTask {
    tx_hex: String,
    start_nonce: u32,
    end_nonce: u32,
    bitwork: String,
}

#[derive(Serialize, Deserialize)]
struct MiningResult {
    success: bool,
    nonce: Option<u32>,
    #[serde(skip)]
    tx: Option<Transaction>,
}
