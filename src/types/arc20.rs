use super::Amount;
use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arc20Config {
    pub tick: String,
    pub mint_amount: Amount,
    pub mint_bitworkc: Option<String>,
    pub mint_bitworkr: Option<String>,
    pub meta: HashMap<String, serde_json::Value>,
}

impl Arc20Config {
    pub fn new(
        tick: String,
        mint_amount: Amount,
    ) -> Result<Self> {
        // Validate ticker
        if !Self::is_valid_ticker(&tick) {
            return Err(Error::InvalidTicker(format!(
                "Invalid ticker format: {}", tick
            )));
        }

        // Validate amount
        if mint_amount.0 == 0 {
            return Err(Error::InvalidAmount("Mint amount cannot be zero".into()));
        }

        Ok(Self {
            tick,
            mint_amount,
            mint_bitworkc: None,
            mint_bitworkr: None,
            meta: HashMap::new(),
        })
    }

    pub fn with_bitworkc(mut self, bitworkc: String) -> Result<Self> {
        if !Self::is_valid_bitwork(&bitworkc) {
            return Err(Error::InvalidBitwork(format!(
                "Invalid bitwork format: {}", bitworkc
            )));
        }
        self.mint_bitworkc = Some(bitworkc);
        Ok(self)
    }

    pub fn with_bitworkr(mut self, bitworkr: String) -> Result<Self> {
        if !Self::is_valid_bitwork(&bitworkr) {
            return Err(Error::InvalidBitwork(format!(
                "Invalid bitwork format: {}", bitworkr
            )));
        }
        self.mint_bitworkr = Some(bitworkr);
        Ok(self)
    }

    pub fn with_meta(mut self, key: String, value: serde_json::Value) -> Self {
        self.meta.insert(key, value);
        self
    }

    // Validate ticker format (3-5 lowercase letters/numbers)
    fn is_valid_ticker(tick: &str) -> bool {
        let len = tick.chars().count();
        if len < 3 || len > 5 {
            return false;
        }
        tick.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    }

    // Validate bitwork format (hex string)
    fn is_valid_bitwork(bitwork: &str) -> bool {
        if bitwork.is_empty() {
            return false;
        }
        bitwork.chars().all(|c| c.is_ascii_hexdigit())
    }

    // Helper methods
    pub fn requires_mining(&self) -> bool {
        self.mint_bitworkc.is_some() || self.mint_bitworkr.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arc20Token {
    pub config: Arc20Config,
    pub minted_supply: Amount,
    pub mint_count: u32,
    pub holders: HashMap<String, Amount>,
    pub mint_bitwork_vec: Option<String>,
    pub mint_phase: u32,
}

impl Arc20Token {
    pub fn new(config: Arc20Config) -> Self {
        Self {
            config,
            minted_supply: Amount::ZERO,
            mint_count: 0,
            holders: HashMap::new(),
            mint_bitwork_vec: None,
            mint_phase: 0,
        }
    }

    pub fn can_mint(&self, _current_height: u32) -> bool {
        true
    }

    pub fn remaining_supply(&self) -> Amount {
        Amount(u64::MAX)
    }

    pub fn add_holder(&mut self, address: String, amount: Amount) -> Result<()> {
        let new_total = self.minted_supply.0.checked_add(amount.0)
            .ok_or_else(|| Error::InvalidAmount("Supply overflow".into()))?;
        
        let entry = self.holders.entry(address).or_insert(Amount::ZERO);
        entry.0 = entry.0.checked_add(amount.0)
            .ok_or_else(|| Error::InvalidAmount("Holder amount overflow".into()))?;
        
        self.minted_supply.0 = new_total;
        self.mint_count += 1;
        
        Ok(())
    }

    pub fn get_holder_balance(&self, address: &str) -> Amount {
        self.holders.get(address).copied().unwrap_or(Amount::ZERO)
    }

    pub fn update_mint_phase(&mut self) {
        self.mint_phase = self.mint_count;
    }
}
