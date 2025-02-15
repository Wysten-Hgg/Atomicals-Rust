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
        web_sys::console::log_1(&format!("Starting mint_ft with tick: {}, amount: {}", tick, mint_amount).into());
        
        // Create config
        web_sys::console::log_1(&"Creating Arc20Config...".into());
        let config = Arc20Config::new(
            tick,
            Amount(mint_amount as u64),
        ).map_err(|e| {
            web_sys::console::error_1(&format!("Failed to create Arc20Config: {}", e).into());
            JsValue::from_str(&e.to_string())
        })?;

        // Add optional bitwork if provided
        web_sys::console::log_1(&format!("Bitwork config: {:?}", mint_bitworkc).into());
        let config = if let Some(bitworkc) = mint_bitworkc {
            config.with_bitworkc(bitworkc)
                .map_err(|e| {
                    web_sys::console::error_1(&format!("Failed to set bitwork: {}", e).into());
                    JsValue::from_str(&e.to_string())
                })?
        } else {
            config
        };

        // Get wallet provider
        web_sys::console::log_1(&format!("Getting wallet provider for type: {}", self.wallet_type).into());
        let result = match self.wallet_type.as_str() {
            "unisat" => {
                web_sys::console::log_1(&"Initializing UniSat wallet...".into());
                let wallet = UnisatProvider::new()
                    .map_err(|e| {
                        web_sys::console::error_1(&format!("Failed to initialize UniSat wallet: {}", e).into());
                        JsValue::from_str(&format!("Failed to initialize UniSat wallet: {}", e))
                    })?;
                mint_ft(&wallet, config, None).await
            }
            "wizz" => {
                web_sys::console::log_1(&"Initializing Wizz wallet...".into());
                let wallet = WizzProvider::new()
                    .map_err(|e| {
                        web_sys::console::error_1(&format!("Failed to initialize Wizz wallet: {}", e).into());
                        JsValue::from_str(&format!("Failed to initialize Wizz wallet: {}", e))
                    })?;
                mint_ft(&wallet, config, None).await
            }
            _ => {
                let err = "Unsupported wallet type";
                web_sys::console::error_1(&err.into());
                return Err(JsValue::from_str(err));
            }
        };

        // Return transaction ID
        match result {
            Ok(tx) => {
                let txid = tx.txid();
                web_sys::console::log_1(&format!("Successfully minted FT with txid: {}", txid).into());
                Ok(txid)
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to mint FT: {}", e).into());
                Err(JsValue::from_str(&format!("Failed to mint FT: {}", e)))
            },
        }
    }
}
