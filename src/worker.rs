use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use bitcoin::Transaction;
use serde::{Serialize, Deserialize};
use crate::errors::Result;

#[derive(Serialize, Deserialize)]
pub struct MiningTask {
    pub tx: Transaction,
    pub start_nonce: u32,
    pub end_nonce: u32,
    pub bitwork_prefix: String,
}

#[derive(Serialize, Deserialize)]
pub struct MiningResult {
    pub success: bool,
    pub nonce: Option<u32>,
    pub tx: Option<Transaction>,
}

#[wasm_bindgen]
pub fn worker_entry() -> Result<()> {
    let worker = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    
    let closure = Closure::wrap(Box::new(move |e: MessageEvent| -> Result<()> {
        let data = e.data();
        let task: MiningTask = serde_wasm_bindgen::from_value(data)?;
        
        let result = mine_range(task);
        
        worker.post_message(&serde_wasm_bindgen::to_value(&result)?)?;
        Ok(())
    }) as Box<dyn FnMut(MessageEvent) -> Result<()>>);

    worker.set_onmessage(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    
    Ok(())
}

fn mine_range(task: MiningTask) -> MiningResult {
    let mut tx = task.tx;
    
    for nonce in task.start_nonce..task.end_nonce {
        if let Some(input) = tx.input.get_mut(0) {
            input.sequence = bitcoin::Sequence(nonce);
        }
        
        let txid = tx.txid().to_string();
        if txid.starts_with(&task.bitwork_prefix) {
            return MiningResult {
                success: true,
                nonce: Some(nonce),
                tx: Some(tx),
            };
        }
    }
    
    MiningResult {
        success: false,
        nonce: None,
        tx: None,
    }
}
