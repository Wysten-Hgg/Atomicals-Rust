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

        // 将输入字符串转换为数字
        let num = match input.parse::<u32>() {
            Ok(n) => n,
            Err(_) => {
                web_sys::console::log_1(&"Invalid number format, using default difficulty".into());
                return Self {
                    prefix: "0000".to_string(),
                    ext: None,
                    difficulty: 16,
                };
            }
        };

        // 根据输入数字确定前缀0的个数
        // 例如：输入"8888"，我们要求8个前导0
        let zeros = if num <= 10 {
            1  // 如果输入1-10，要求1个0
        } else if num <= 100 {
            2  // 如果输入11-100，要求2个0
        } else if num <= 1000 {
            3  // 如果输入101-1000，要求3个0
        } else if num <= 10000 {
            4  // 如果输入1001-10000，要求4个0
        } else {
            5  // 更大的数字，要求5个0
        };

        let prefix = "0".repeat(zeros);
        let difficulty = zeros * 4; // 每个十六进制0代表4位二进制

        web_sys::console::log_1(&format!(
            "Created BitworkInfo with {} leading zeros (prefix: {}, difficulty: {})",
            zeros, prefix, difficulty
        ).into());

        Self {
            prefix,
            ext: None,
            difficulty: difficulty as u32,
        }
    }

    pub fn with_ext(mut self, ext: String) -> Self {
        self.ext = Some(ext);
        self
    }

    pub fn matches(&self, txid: &str) -> bool {
        // 检查txid是否以指定数量的0开头
        let required_zeros = self.prefix.len();
        if txid.len() < required_zeros {
            return false;
        }

        // 检查前缀中的每个字符是否为"0"
        if !txid[..required_zeros].chars().all(|c| c == '0') {
            return false;
        }

        // 检查扩展要求（如果有）
        if let Some(ext) = &self.ext {
            let ext_pos = required_zeros;
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
