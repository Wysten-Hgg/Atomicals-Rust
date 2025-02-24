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
    pub fn new(input: String) -> Self {
        web_sys::console::log_1(&format!("Converting bitwork input: {}", input).into());
        
        // 如果输入为空，使用默认值
        if input.is_empty() {
            return Self {
                prefix: "0000".to_string(),
                ext: None,
                difficulty: 16,
            };
        }

        // 解析输入字符串，支持扩展要求（例如 "aabbcc:ext"）
        let parts: Vec<&str> = input.split(':').collect();
        let (prefix, ext) = match parts.len() {
            1 => (parts[0].to_string(), None),
            2 => (parts[0].to_string(), Some(parts[1].to_string())),
            _ => {
                web_sys::console::log_1(&"Invalid bitwork format, using default values".into());
                return Self {
                    prefix: "0000".to_string(),
                    ext: None,
                    difficulty: 16,
                };
            }
        };

        // 验证前缀是否有效（仅包含十六进制字符）
        if !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
            web_sys::console::log_1(&"Invalid prefix, using default values".into());
            return Self {
                prefix: "0000".to_string(),
                ext: None,
                difficulty: 16,
            };
        }

        // 计算难度（基于前缀长度）
        let difficulty = prefix.len() as u32 * 4;

        web_sys::console::log_1(&format!(
            "Created BitworkInfo with prefix: {}, ext: {:?}, difficulty: {}",
            prefix, ext, difficulty
        ).into());

        Self {
            prefix,
            ext,
            difficulty,
        }
    }

    pub fn with_ext(mut self, ext: String) -> Self {
        self.ext = Some(ext);
        self
    }

    pub fn matches(&self, txid: &str) -> bool {
        // 检查txid是否以指定前缀开头
        let required_prefix_len = self.prefix.len();
        if txid.len() < required_prefix_len {
            return false;
        }

        if !txid.starts_with(&self.prefix) {
            return false;
        }

        // 检查扩展要求（如果有）
        if let Some(ext) = &self.ext {
            let ext_pos = required_prefix_len;
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