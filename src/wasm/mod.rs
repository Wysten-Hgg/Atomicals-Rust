use wasm_bindgen::prelude::*;
use crate::types::{Arc20Config, MintConfig, RealmConfig};
use crate::operations::{mint_ft, mining::MiningOptions, mint_realm};
use crate::wallet::web::WizzProvider;
use std::collections::HashMap;
use serde_json::Value;

#[wasm_bindgen(js_name = "AtomicalsWasm")]
pub struct Atomicals {
    wallet: WizzProvider,
}

#[wasm_bindgen(js_class = "AtomicalsWasm")]
impl Atomicals {
    #[wasm_bindgen(constructor)]
    pub fn try_new() -> std::result::Result<Atomicals, JsValue> {
        let wallet = WizzProvider::try_new()?;
        Ok(Atomicals { wallet })
    }

    #[wasm_bindgen]
    pub async fn mint_ft(
        &self,
        tick: String,
        mint_amount: u64,
        bitwork_c: Option<String>,
        bitwork_r: Option<String>,
        num_workers: Option<u32>,
        batch_size: Option<u32>,
    ) -> std::result::Result<JsValue, JsValue> {
        let config = Arc20Config {
            tick,
            mint_amount: crate::types::Amount(mint_amount),
            mint_bitworkc: bitwork_c,
            mint_bitworkr: bitwork_r,
            meta: HashMap::new(),
        };

        let mining_options = if num_workers.is_some() || batch_size.is_some() {
            Some(MiningOptions {
                num_workers: num_workers.unwrap_or(4),
                batch_size: batch_size.unwrap_or(1000),
            })
        } else {
            None
        };

        let result = mint_ft::mint_ft(&self.wallet, config, mining_options).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn mint_realm(
        &self,
        name: String,
        sats_output: u64,
        bitwork_c: Option<String>,
        bitwork_r: Option<String>,
        container: Option<String>,
        parent: Option<String>,
        parent_owner: Option<String>,
        num_workers: Option<u32>,
        batch_size: Option<u32>,
    ) -> std::result::Result<JsValue, JsValue> {
        let config = RealmConfig {
            name,
            bitworkc: bitwork_c,
            bitworkr: bitwork_r,
            container,
            parent,
            parent_owner,
            sats_output,
        };

        let mining_options = if num_workers.is_some() || batch_size.is_some() {
            Some(MiningOptions {
                num_workers: num_workers.unwrap_or(4),
                batch_size: batch_size.unwrap_or(1000),
            })
        } else {
            None
        };

        let result = mint_realm(&self.wallet, config, mining_options)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}
