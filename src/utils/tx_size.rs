use bitcoin::{Transaction, TxIn, TxOut, ScriptBuf};
use log;

pub struct TransactionSize {
    pub base_size: usize,    // 非见证数据大小
    pub witness_size: usize, // 见证数据大小
    pub total_vsize: usize,  // 虚拟大小 (vsize)
}

impl TransactionSize {
    pub fn new(base_size: usize, witness_size: usize) -> Self {
        // vsize = (base_size * 4 + witness_size) / 4
        let total_vsize = (base_size * 4 + witness_size + 3) / 4;
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
        ScriptType::P2PKH => TransactionSize::new(148, 0),    // 非隔离见证
        ScriptType::P2WPKH => TransactionSize::new(68, 107),  // 原生隔离见证
        ScriptType::P2SH_P2WPKH => TransactionSize::new(91, 107), // 兼容隔离见证
    }
}

// 计算不同类型输出的大小
pub fn get_output_size(script_type: &ScriptType) -> usize {
    match script_type {
        ScriptType::P2PKH => 34,
        ScriptType::P2WPKH => 31,
        ScriptType::P2SH_P2WPKH => 32,
    }
}

// 计算完整交易的大小
pub fn calculate_tx_size(
    input_types: &[ScriptType],
    output_types: &[ScriptType],
    has_op_return: bool,
) -> TransactionSize {
    // 基础交易开销：版本(4) + 输入数量(1-9) + 输出数量(1-9) + locktime(4)
    let mut base_size = 10;
    let mut witness_size = 0;

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
        base_size += 40; // 估算值，实际大小取决于 OP_RETURN 数据
    }

    TransactionSize::new(base_size, witness_size)
}

#[derive(Debug, Clone, Copy)]
pub enum ScriptType {
    P2PKH,      // Pay to Public Key Hash
    P2WPKH,     // Native SegWit
    P2SH_P2WPKH, // Nested SegWit
}

impl ScriptType {
    pub fn from_script(script: &ScriptBuf) -> Option<Self> {
        if script.is_p2pkh() {
            Some(ScriptType::P2PKH)
        } else if script.is_v0_p2wpkh() {
            Some(ScriptType::P2WPKH)
        } else if script.is_p2sh() && script.as_bytes().len() == 23 {
            // P2SH-P2WPKH 脚本的长度应该是 23 字节
            // 这是一个更准确的检查，但仍然是启发式的
            Some(ScriptType::P2SH_P2WPKH)
        } else {
            log::warn!("Unsupported script type: {}", script);
            Some(ScriptType::P2WPKH)  // 默认使用 P2WPKH，这样至少能继续运行
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
        assert_eq!(size.base_size, 10 + (68 * 2) + (31 * 2));
        assert_eq!(size.witness_size, 107 * 2);
        
        // 验证 vsize 计算
        let expected_vsize = (size.base_size * 4 + size.witness_size + 3) / 4;
        assert_eq!(size.total_vsize, expected_vsize);
    }
}
