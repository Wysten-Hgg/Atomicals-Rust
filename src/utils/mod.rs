use crate::errors::{Error, Result};
use bitcoin::Transaction;

pub mod script_builder;
pub use script_builder::*;

pub mod tx_size;

pub fn validate_ticker(ticker: &str) -> bool {
    // Ticker rules:
    // 1. Length between 1 and 21 characters
    // 2. Only alphanumeric characters
    // 3. Must start with a letter
    // 4. Case insensitive
    
    if ticker.len() < 1 || ticker.len() > 21 {
        return false;
    }
    
    let first_char = ticker.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() {
        return false;
    }
    
    ticker.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn estimate_tx_size(tx: &Transaction) -> Result<usize> {
    // 一个简单的估算，可以根据需要调整
    let base_size = 10; // 版本 + 时间锁
    let input_size = tx.input.len() * 150; // 每个输入约150字节
    let output_size = tx.output.len() * 34; // 每个输出约34字节
    
    Ok(base_size + input_size + output_size)
}

pub fn validate_ticker_new(ticker: &str) -> Result<()> {
    if ticker.is_empty() || ticker.len() > 21 {
        return Err(Error::InvalidTicker("Ticker must be between 1 and 21 characters".into()));
    }
    
    if !ticker.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::InvalidTicker("Ticker can only contain alphanumeric characters and underscores".into()));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_ticker() {
        assert!(validate_ticker("TEST"));
        assert!(validate_ticker("Test123"));
        assert!(!validate_ticker("123Test"));
        assert!(!validate_ticker("Test@123"));
        assert!(!validate_ticker(""));
        assert!(!validate_ticker("ThisIsAVeryLongTickerName"));
    }

    #[test]
    fn test_estimate_tx_size() {
        let tx = Transaction {
            version: 2,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        };
        
        assert_eq!(estimate_tx_size(&tx).unwrap(), 10);
    }
}
