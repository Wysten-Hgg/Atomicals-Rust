use wasm_bindgen::prelude::*;
use bitcoin::Transaction;
use serde::{Serialize, Deserialize};
use crate::types::mint::BitworkInfo;
use bitcoin::consensus;
use hex;

#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmTransaction {
    hex: String,
}

#[wasm_bindgen]
impl WasmTransaction {
    #[wasm_bindgen(constructor)]
    pub fn new(hex: String) -> Self {
        Self { hex }
    }

    #[wasm_bindgen]
    pub fn to_hex(&self) -> String {
        self.hex.clone()
    }

    #[wasm_bindgen]
    pub fn from_hex(hex: String) -> Self {
        Self { hex }
    }

    #[wasm_bindgen]
    pub fn set_sequence(&mut self, nonce: u32) -> bool {
        if let Some(mut tx) = self.to_transaction() {
            if !tx.input.is_empty() {
                tx.input[0].sequence = bitcoin::transaction::Sequence(nonce);
                self.hex = hex::encode(consensus::serialize(&tx));
                return true;
            }
        }
        false
    }
}

impl WasmTransaction {
    pub fn to_transaction(&self) -> Option<Transaction> {
        hex::decode(&self.hex)
            .ok()
            .and_then(|bytes| consensus::deserialize(&bytes).ok())
    }

    pub fn from_transaction(tx: &Transaction) -> Self {
        Self {
            hex: hex::encode(consensus::serialize(tx)),
        }
    }
}

#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmBitworkInfo {
    difficulty: String,
    prefix: String,
    ext: Option<String>,
}

#[wasm_bindgen]
impl WasmBitworkInfo {
    #[wasm_bindgen(constructor)]
    pub fn new(difficulty: String, prefix: String) -> Self {
        Self {
            difficulty,
            prefix,
            ext: None,
        }
    }

    #[wasm_bindgen]
    pub fn get_difficulty(&self) -> String {
        self.difficulty.clone()
    }

    #[wasm_bindgen]
    pub fn get_prefix(&self) -> String {
        self.prefix.clone()
    }

    #[wasm_bindgen]
    pub fn get_ext(&self) -> Option<String> {
        self.ext.clone()
    }

    #[wasm_bindgen]
    pub fn set_ext(&mut self, ext: Option<String>) {
        self.ext = ext;
    }
}

impl WasmBitworkInfo {
    pub fn to_bitwork_info(&self) -> BitworkInfo {
        BitworkInfo {
            difficulty: self.difficulty.parse().unwrap_or(16),
            prefix: self.prefix.clone(),
            ext: self.ext.clone(),
        }
    }

    pub fn from_bitwork_info(bitwork: &BitworkInfo) -> Self {
        Self {
            difficulty: bitwork.difficulty.to_string(),
            prefix: bitwork.prefix.clone(),
            ext: bitwork.ext.clone(),
        }
    }
}
