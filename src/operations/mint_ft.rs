use crate::types::{AtomicalsTx, arc20::{Arc20Config, Arc20Token}};
use crate::types::mint::{BitworkInfo, MintConfig, MintResult};
use crate::errors::{Error, Result};
use crate::wallet::{WalletProvider, Utxo};
use crate::types::wasm::{WasmTransaction, WasmBitworkInfo};
use crate::operations::mining::{mine_transaction, MiningOptions, MiningResult};
use crate::utils::tx_size::{self, ScriptType};
use bitcoin::{
    Amount, Network, Transaction, TxIn, TxOut, Sequence,
    psbt::Psbt, ScriptBuf, Address,
    transaction::Version,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
}

fn select_utxos(utxos: &[Utxo], target_amount: Amount, fee_rate: f64) -> Result<(Vec<Utxo>, Amount)> {
    let mut selected_utxos = Vec::new();
    let mut total_amount = Amount::from_sat(0);
    
    // 预计输出的脚本类型（假设都是 P2WPKH）
    let output_types = vec![
        ScriptType::P2WPKH, // commit tx 的第一个输出
        ScriptType::P2WPKH, // commit tx 的第二个输出
    ];
    
    for utxo in utxos {
        selected_utxos.push(utxo.clone());
        
        // 处理 Amount 加法
        total_amount = match total_amount.checked_add(utxo.txout.value) {
            Some(amount) => amount,
            None => return Err(Error::TransactionError("Amount overflow".into())),
        };
            
        // 获取输入的脚本类型
        let script_type = ScriptType::from_script(&utxo.txout.script_pubkey)
            .ok_or_else(|| Error::TransactionError("Unsupported script type".into()))?;
            
        // 构建输入类型列表
        let input_types: Vec<ScriptType> = selected_utxos.iter()
            .map(|u| ScriptType::from_script(&u.txout.script_pubkey)
                .unwrap_or(ScriptType::P2WPKH))
            .collect();
            
        // 计算当前交易大小
        let tx_size = tx_size::calculate_tx_size(
            &input_types,
            &output_types,
            true  // 有 OP_RETURN 输出
        );
        
        // 计算预估手续费
        let fee = Amount::from_sat((tx_size.total_vsize as f64 * fee_rate) as u64);
        
        // 检查是否已经收集了足够的金额
        match total_amount.checked_sub(fee) {
            Some(remaining) => {
                if remaining >= target_amount {
                    return Ok((selected_utxos, fee));
                }
            }
            None => continue, // 如果减法溢出，继续尝试下一个 UTXO
        }
    }
    
    Err(Error::InvalidAmount("Not enough funds to cover amount and fees".into()))
}

pub async fn mint_ft<W: WalletProvider>(
    wallet: &W,
    config: Arc20Config,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    log!("Starting mint_ft operation...");

    // 获取钱包地址
    let address_str = wallet.get_address().await?;
    let address = Address::from_str(&address_str)
        .map_err(|e| Error::AddressError(e.to_string()))?
        .require_network(Network::Testnet)
        .map_err(|e| Error::NetworkError(e.to_string()))?;

    // 获取 UTXO 列表和网络费率
    let utxos = wallet.get_utxos().await?;
    let fee_rate = wallet.get_network_fee_rate().await?;
    
    // 选择合适的 UTXO 并计算手续费
    let (selected_utxos, fee) = select_utxos(
        &utxos,
        Amount::from_sat(config.mint_amount.0),
        fee_rate
    )?;
    
    // 创建交易输入
    let inputs: Vec<TxIn> = selected_utxos.iter()
        .map(|utxo| TxIn {
            previous_output: utxo.outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Default::default(),
        })
        .collect();
        
    // 计算总输入金额
    let total_input = selected_utxos.iter()
        .try_fold(Amount::from_sat(0), |acc, utxo| {
            acc.checked_add(utxo.txout.value)
                .ok_or_else(|| Error::TransactionError("Amount overflow".into()))
        })?;
    
    // 计算找零金额
    let target_amount = Amount::from_sat(config.mint_amount.0);
    let change_amount = match total_input.checked_sub(fee) {
        Some(remaining) => match remaining.checked_sub(target_amount) {
            Some(change) => change,
            None => return Err(Error::InvalidAmount("Not enough funds after fees".into())),
        },
        None => return Err(Error::InvalidAmount("Not enough funds to cover fees".into())),
    };

    // 创建 commit 交易
    let mut commit_outputs = vec![
        TxOut {
            value: Amount::from_sat(0),
            script_pubkey: address.script_pubkey(),
        },
        TxOut {
            value: target_amount,
            script_pubkey: address.script_pubkey(),
        },
    ];
    
    // 如果有找零，添加找零输出
    if change_amount > Amount::from_sat(0) {
        commit_outputs.push(TxOut {
            value: change_amount,
            script_pubkey: address.script_pubkey(),
        });
    }

    let commit_tx = Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: inputs,
        output: commit_outputs,
    };

    let mut commit_psbt = Psbt::from_unsigned_tx(commit_tx.clone())
    .map_err(|e| Error::PsbtError(format!("Failed to create commit PSBT: {}", e)))?;

        
    // 添加输入的 UTXO 信息到 PSBT
    for (i, utxo) in selected_utxos.iter().enumerate() {
        commit_psbt.inputs[i].witness_utxo = Some(utxo.txout.clone());
    }
    
    log!("Created commit PSBT");

    // 创建 reveal 交易
    let reveal_tx = Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![],
        output: vec![
            TxOut {
                value: target_amount,
                script_pubkey: address.script_pubkey(),
            },
        ],
    };

   let mut reveal_psbt = Psbt::from_unsigned_tx(reveal_tx.clone())
    .map_err(|e| Error::PsbtError(format!("Failed to create reveal PSBT: {}", e)))?;
    log!("Created reveal PSBT");

    // Mine transactions if needed
    if let Some(ref mining_opts) = mining_options {
        if let Some(ref bitworkc) = config.mint_bitworkc {
            log!("Mining commit transaction...");
            let mining_result = mine_transaction(
                WasmTransaction::from_transaction(&commit_tx),
                WasmBitworkInfo::from_bitwork_info(&BitworkInfo::new(bitworkc.clone())),
                MiningOptions::new(),
            ).await?;

            let mining_result: MiningResult = serde_wasm_bindgen::from_value(mining_result)?;
            if mining_result.success {
                if let Some(mined_tx) = mining_result.get_transaction() {
                    commit_psbt = Psbt::from_unsigned_tx(mined_tx)
                        .map_err(|e| Error::PsbtError(format!("Failed to create PSBT after mining commit tx: {}", e)))?;
                    log!("Commit transaction mined successfully");
                }
            } else {
                return Err(Error::MiningError("Failed to mine commit transaction".into()));
            }
        }

        if let Some(ref bitworkr) = config.mint_bitworkr {
            log!("Mining reveal transaction...");
            let mining_result = mine_transaction(
                WasmTransaction::from_transaction(&reveal_tx),
                WasmBitworkInfo::from_bitwork_info(&BitworkInfo::new(bitworkr.clone())),
                MiningOptions::new(),
            ).await?;

            let mining_result: MiningResult = serde_wasm_bindgen::from_value(mining_result)?;
            if mining_result.success {
                if let Some(mined_tx) = mining_result.get_transaction() {
                    reveal_psbt = Psbt::from_unsigned_tx(mined_tx)
                        .map_err(|e| Error::PsbtError(format!("Failed to create PSBT after mining reveal tx: {}", e)))?;
                    log!("Reveal transaction mined successfully");
                }
            } else {
                return Err(Error::MiningError("Failed to mine reveal transaction".into()));
            }
        }
    }

    // Sign transactions
    log!("Signing transactions...");
    let signed_commit = wallet.sign_psbt(commit_psbt).await?;
    let signed_reveal = wallet.sign_psbt(reveal_psbt).await?;

    // Extract transactions
    let commit_tx = signed_commit.extract_tx()
        .map_err(|e| Error::TransactionError(format!("Failed to extract commit tx: {}", e)))?;

    let reveal_tx = signed_reveal.extract_tx()
        .map_err(|e| Error::TransactionError(format!("Failed to extract reveal tx: {}", e)))?;

    // Broadcast commit transaction
    let commit_txid = wallet.broadcast_transaction(commit_tx.clone()).await?;

    // Broadcast reveal transaction
    let reveal_txid = wallet.broadcast_transaction(reveal_tx.clone()).await?;

    // Create AtomicalsTx
    let atomicals_tx = AtomicalsTx::new_with_commit_reveal(
        commit_tx,
        reveal_tx,
        Some(commit_txid),
        Some(reveal_txid),
    );

    Ok(atomicals_tx)
}
