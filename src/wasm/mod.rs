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
        max_supply: u64,
        mint_amount: u64,
        mint_height: u32,
        max_mints: u32,
        mint_bitworkc: Option<String>,
    ) -> Result<String, JsValue> {
        // Create config
        let config = Arc20Config::new(
            tick,
            Amount(max_supply),
            Amount(mint_amount),
            mint_height,
            max_mints,
        ).map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Add bitwork if provided
        let config = if let Some(bitwork) = mint_bitworkc {
            config.with_bitworkc(bitwork)
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        } else {
            config
        };

        // Get wallet provider
        let result = match self.wallet_type.as_str() {
            "unisat" => {
                let wallet = UnisatProvider::new()
                    .map_err(|e| JsValue::from_str(&format!("Failed to initialize UniSat wallet: {}", e)))?;
                mint_ft(&wallet, config, None).await
            }
            "wizz" => {
                let wallet = WizzProvider::new()
                    .map_err(|e| JsValue::from_str(&format!("Failed to initialize Wizz wallet: {}", e)))?;
                mint_ft(&wallet, config, None).await
            }
            _ => return Err(JsValue::from_str("Unsupported wallet type")),
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
