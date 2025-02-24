use bitcoin::{Transaction, TxIn, TxOut, ScriptBuf};
use log;

pub const FEE_BASE_BYTES: f64 = 10.5;
pub const FEE_INPUT_BYTES_BASE: f64 = 57.5;
pub const FEE_OUTPUT_BYTES_BASE: f64 = 43.0;
pub const REVEAL_INPUT_BYTES_BASE: f64 = 66.0; // 41 + 25 (witness weight/4)

pub struct TransactionSize {
    pub base_size: f64,    // 非见证数据大小
    pub witness_size: f64, // 见证数据大小
    pub total_vsize: f64,  // 虚拟大小 (vsize)
}

impl TransactionSize {
    pub fn new(base_size: f64, witness_size: f64) -> Self {
        // vsize = (base_size * 4 + witness_size) / 4
        let total_vsize = (base_size * 4.0 + witness_size + 3.0) / 4.0;
        Self {
            base_size,
            witness_size,
            total_vsize,
        }
    }
}

// 计算不同类型输入的大小
pub fn get_input_size(script_type: &ScriptType) -> TransactionSize {
    match script_type {
        ScriptType::P2PKH => TransactionSize::new(148.0, 0.0),    // 非隔离见证
        ScriptType::P2WPKH => TransactionSize::new(68.0, 107.0),  // 原生隔离见证
        ScriptType::P2TR => TransactionSize::new(108.0, 108.0),  // Taproot
    }
}

// 计算不同类型输出的大小
pub fn get_output_size(script_type: &ScriptType) -> f64 {
    match script_type {
        ScriptType::P2PKH => 34.0,
        ScriptType::P2WPKH => 31.0,
        ScriptType::P2TR => 34.0,
    }
}

// 计算完整交易的大小
pub fn calculate_tx_size(
    input_types: &[ScriptType],
    output_types: &[ScriptType],
    has_op_return: bool,
) -> TransactionSize {
    // 基础交易开销：版本(4) + 输入数量(1-9) + 输出数量(1-9) + locktime(4)
    let mut base_size = 10.0;
    let mut witness_size = 0.0;

    // 添加输入大小
    for input_type in input_types {
        let size = get_input_size(input_type);
        base_size += size.base_size;
        witness_size += size.witness_size;
    }

    // 添加输出大小
    for output_type in output_types {
        base_size += get_output_size(output_type);
    }

    // 如果有 OP_RETURN，添加额外大小
    if has_op_return {
        base_size += 40.0; // 估算值，实际大小取决于 OP_RETURN 数据
    }

    TransactionSize::new(base_size, witness_size)
}

pub fn calculate_reveal_size(
    input_num: usize,
    output_num: usize,
    hash_lock_script_len: usize,
) -> f64 {
    // 计算hash_lock_script的compact size字节数
    let hash_lock_compact_size_bytes = if hash_lock_script_len <= 252 {
        1
    } else if hash_lock_script_len <= 0xffff {
        3
    } else if hash_lock_script_len <= 0xffffffff {
        5
    } else {
        9
    };

    FEE_BASE_BYTES +
        // Reveal input
        REVEAL_INPUT_BYTES_BASE +
        (hash_lock_compact_size_bytes as f64 + hash_lock_script_len as f64) / 4.0 +
        // Additional inputs
        (input_num as f64 * FEE_INPUT_BYTES_BASE) +
        // Outputs
        (output_num as f64 * FEE_OUTPUT_BYTES_BASE)
}

pub fn calculate_commit_size(
    input_num: usize,
    output_num: usize,
) -> f64 {
    FEE_BASE_BYTES +
        (input_num as f64 * FEE_INPUT_BYTES_BASE) +
        (output_num as f64 * FEE_OUTPUT_BYTES_BASE)
}

#[derive(Debug, Clone, Copy)]
pub enum ScriptType {
    P2PKH,      // Pay to Public Key Hash
    P2WPKH,     // Native SegWit
    P2TR,       // Taproot
}

impl ScriptType {
    pub fn from_script(script: &bitcoin::ScriptBuf) -> Option<Self> {
        if script.is_p2pkh() {
            Some(ScriptType::P2PKH)
        } else if script.is_v0_p2wpkh() {
            Some(ScriptType::P2WPKH)
        } else if script.is_v1_p2tr() {
            Some(ScriptType::P2TR)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2wpkh_transaction_size() {
        // 测试一个典型的 P2WPKH 交易（2个输入，2个输出）
        let input_types = vec![ScriptType::P2WPKH, ScriptType::P2WPKH];
        let output_types = vec![ScriptType::P2WPKH, ScriptType::P2WPKH];
        
        let size = calculate_tx_size(&input_types, &output_types, false);
        
        // 验证计算结果
        assert_eq!(size.base_size, 10.0 + (68.0 * 2.0) + (31.0 * 2.0));
        assert_eq!(size.witness_size, 107.0 * 2.0);
        
        // 验证 vsize 计算
        let expected_vsize = (size.base_size * 4.0 + size.witness_size + 3.0) / 4.0;
        assert_eq!(size.total_vsize, expected_vsize);
    }
}
