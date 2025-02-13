use super::{Arc20Config, Arc20Token};
use bitcoin::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintConfig {
    pub arc20: Arc20Config,
    pub recipient_address: String,
    pub fee_rate: u64,
}

#[derive(Debug)]
pub struct MintResult {
    pub token: Arc20Token,
    pub transaction: Transaction,
    pub txid: String,
}

#[derive(Debug, Clone)]
pub struct BitworkInfo {
    pub prefix: String,
    pub ext: Option<String>,
    pub difficulty: u32,
}

impl BitworkInfo {
    pub fn new(prefix: String) -> Self {
        let difficulty = prefix.len() as u32 * 4; // Calculate difficulty before moving prefix
        Self {
            prefix: prefix.clone(), // Clone prefix before moving it
            ext: None,
            difficulty,
        }
    }

    pub fn with_ext(mut self, ext: String) -> Self {
        self.ext = Some(ext);
        self
    }

    pub fn matches(&self, txid: &str) -> bool {
        if !txid.starts_with(&self.prefix) {
            return false;
        }

        if let Some(ext) = &self.ext {
            let ext_pos = self.prefix.len();
            if ext_pos + ext.len() > txid.len() {
                return false;
            }
            if &txid[ext_pos..ext_pos + ext.len()] != ext {
                return false;
            }
        }

        true
    }
}
