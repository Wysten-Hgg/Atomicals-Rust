use wasm_bindgen::prelude::*;
use crate::operations::mint_ft::mint_ft;
use crate::types::{Amount, Arc20Config};
use crate::wallet::web::{UnisatProvider, WizzProvider};

#[wasm_bindgen]
pub struct AtomicalsWasm {
    wallet_type: String,
}

#[wasm_bindgen]
impl AtomicalsWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(wallet_type: String) -> Self {
        Self { wallet_type }
    }

    #[wasm_bindgen]
    pub async fn mint_ft(
        &self,
        tick: String,
        mint_amount: f64,
        mint_bitworkc: Option<String>,
    ) -> Result<String, JsValue> {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Starting mint_ft with tick: {}, amount: {}, bitworkc: {:?}",
            tick, mint_amount, mint_bitworkc
        )));

        // Validate input
        if mint_amount <= 0.0 || mint_amount.fract() != 0.0 {
            web_sys::console::error_1(&JsValue::from_str("Invalid mint amount"));
            return Err(JsValue::from_str("Mint amount must be a positive integer"));
        }

        // Create config
        web_sys::console::log_1(&JsValue::from_str("Creating Arc20Config..."));
        let config = match Arc20Config::new(
            tick,
            Amount(mint_amount as u64),
        ) {
            Ok(cfg) => cfg,
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Failed to create Arc20Config: {}", e
                )));
                return Err(JsValue::from_str(&e.to_string()));
            }
        };

        // Add bitwork if provided
        let config = if let Some(bitwork) = mint_bitworkc {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Adding bitwork: {}", bitwork
            )));
            match config.with_bitworkc(bitwork) {
                Ok(cfg) => cfg,
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Failed to add bitwork: {}", e
                    )));
                    return Err(JsValue::from_str(&e.to_string()));
                }
            }
        } else {
            web_sys::console::log_1(&JsValue::from_str("No bitwork provided"));
            config
        };

        // Get wallet provider
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Using wallet type: {}", self.wallet_type
        )));
        let result = match self.wallet_type.as_str() {
            "unisat" => {
                let wallet = match UnisatProvider::new() {
                    Ok(w) => w,
                    Err(e) => {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to initialize UniSat wallet: {}", e
                        )));
                        return Err(JsValue::from_str(&format!("Failed to initialize UniSat wallet: {}", e)));
                    }
                };
                mint_ft(&wallet, config, None).await
            }
            "wizz" => {
                let wallet = match WizzProvider::new() {
                    Ok(w) => w,
                    Err(e) => {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to initialize Wizz wallet: {}", e
                        )));
                        return Err(JsValue::from_str(&format!("Failed to initialize Wizz wallet: {}", e)));
                    }
                };
                mint_ft(&wallet, config, None).await
            }
            _ => {
                web_sys::console::error_1(&JsValue::from_str("Unsupported wallet type"));
                return Err(JsValue::from_str("Unsupported wallet type"));
            }
        };

        // Return transaction ID
        match result {
            Ok(tx) => {
                let txid = tx.txid();
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Successfully minted FT with txid: {}", txid
                )));
                Ok(txid)
            },
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Failed to mint FT: {}", e
                )));
                Err(JsValue::from_str(&format!("Failed to mint FT: {}", e)))
            },
        }
    }
}
