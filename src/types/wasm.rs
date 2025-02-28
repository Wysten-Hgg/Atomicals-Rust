use wasm_bindgen::prelude::*;
use bitcoin::Transaction;
use serde::{Serialize, Deserialize};
use crate::types::mint::BitworkInfo;
use bitcoin::consensus;
use hex;
use wasm_bindgen::JsValue;
use crate::RealmConfig;

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

#[wasm_bindgen]
#[derive(Clone,Serialize, Deserialize)]
pub struct WasmRealmConfig {
    name: String,
    bitworkc: Option<String>,
    bitworkr: Option<String>,
    container: Option<String>,
    parent: Option<String>,
    parent_owner: Option<String>,
    sats_output: u64,
}

#[wasm_bindgen]
impl WasmRealmConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> Self {
        Self {
            name,
            bitworkc: None,
            bitworkr: None,
            container: None,
            parent: None,
            parent_owner: None,
            sats_output: 546,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn bitworkc(&self) -> Option<String> {
        self.bitworkc.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn bitworkr(&self) -> Option<String> {
        self.bitworkr.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn container(&self) -> Option<String> {
        self.container.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn parent(&self) -> Option<String> {
        self.parent.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn parent_owner(&self) -> Option<String> {
        self.parent_owner.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn sats_output(&self) -> u64 {
        self.sats_output
    }

    #[wasm_bindgen]
    pub fn with_bitworkc(mut self, bitworkc: String) -> Self {
        self.bitworkc = Some(bitworkc);
        self
    }

    #[wasm_bindgen]
    pub fn with_bitworkr(mut self, bitworkr: String) -> Self {
        self.bitworkr = Some(bitworkr);
        self
    }

    #[wasm_bindgen]
    pub fn with_container(mut self, container: String) -> Self {
        self.container = Some(container);
        self
    }

    #[wasm_bindgen]
    pub fn with_parent(mut self, parent: String, parent_owner: Option<String>) -> Self {
        self.parent = Some(parent);
        self.parent_owner = parent_owner;
        self
    }

    #[wasm_bindgen]
    pub fn with_sats_output(mut self, sats: u64) -> Self {
        self.sats_output = sats;
        self
    }

    #[wasm_bindgen]
    pub fn validate(&self) -> Result<(), JsValue> {
        let realm_config: RealmConfig = RealmConfig::from(self.clone());
        realm_config.validate_name()
            .map_err(|e| JsValue::from_str(&format!("Invalid realm name: {}", e)))
    }
}
impl From<WasmRealmConfig> for RealmConfig {
    fn from(config: WasmRealmConfig) -> Self {
        RealmConfig {
            name: config.name,
            bitworkc: config.bitworkc,
            bitworkr: config.bitworkr,
            container: config.container,
            parent: config.parent,
            parent_owner: config.parent_owner,
            sats_output: config.sats_output,
        }
    }
}

impl From<RealmConfig> for WasmRealmConfig {
    fn from(config: RealmConfig) -> Self {
        WasmRealmConfig {
            name: config.name,
            bitworkc: config.bitworkc,
            bitworkr: config.bitworkr,
            container: config.container,
            parent: config.parent,
            parent_owner: config.parent_owner,
            sats_output: config.sats_output,
        }
    }
}
