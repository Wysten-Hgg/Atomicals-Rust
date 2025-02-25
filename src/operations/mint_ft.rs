use crate::types::{AtomicalsTx, arc20::{Arc20Config, Arc20Token}};
use crate::types::mint::{BitworkInfo, MintConfig, MintResult};
use crate::errors::{Error, Result};
use crate::wallet::{WalletProvider, Utxo};
use crate::types::wasm::{WasmTransaction, WasmBitworkInfo};
use crate::operations::mining::{mine_transaction, MiningOptions, MiningResult};
use crate::utils::tx_size::{self, ScriptType};
use bitcoin::{
    Amount, Network, Transaction, TxIn, TxOut, Sequence,
    psbt::Psbt, ScriptBuf, Address, taproot::{TapTree, TaprootBuilder, LeafVersion},
    transaction::Version, key::XOnlyPublicKey, secp256k1::Secp256k1,
    OutPoint,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;
use serde_cbor;
use serde::Serialize;
use crate::utils::script::append_mint_update_reveal_script;
use crate::utils::script::time_nonce;
use crate::utils::script::cbor;
use web_sys;
use js_sys;
use wasm_bindgen_futures;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
}
#[derive(Debug, Serialize)]
pub struct PayloadWrapper {
	pub args: Payload,
}
#[derive(Debug, Serialize)]
pub struct Payload {
	pub bitworkc: String,
	pub mint_ticker: String,
	pub nonce: u64,
	pub time: u64,
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

pub async fn prepare_commit_reveal_config(
    op_type: &str,
    child_node_xonly_pubkey: &XOnlyPublicKey,
    atomicals_payload: &[u8],
    network: Network
) -> Result<(ScriptBuf, Address)> {
    // 构建 Taproot 脚本
    let script = append_mint_update_reveal_script(op_type, &child_node_xonly_pubkey, atomicals_payload)?;
    
    // 创建 TaprootBuilder
    let mut builder = TaprootBuilder::new();
    builder = builder.add_leaf(0, script.clone())?;
    
    let secp = Secp256k1::new();
    
    // 构建 Taproot 输出并验证脚本是否在路径中
    let taproot_info = builder.finalize(&secp, *child_node_xonly_pubkey)?;
    if taproot_info.merkle_root().is_none() {
        return Err(Error::TransactionError("Failed to add script to Taproot path".into()));
    }
    
    let tr_script = ScriptBuf::new_v1_p2tr(&secp, *child_node_xonly_pubkey, Some(taproot_info.merkle_root().unwrap()));
    let tr_address = Address::from_script(&tr_script, network)?;
    
    Ok((script, tr_address))
}

pub async fn mint_ft<W: WalletProvider>(
    wallet: &W,
    config: Arc20Config,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    log!("Starting mint_ft operation...");

    // 获取钱包公钥和地址
    let address_str = wallet.get_address().await?;
    let address = Address::from_str(&address_str)
        .map_err(|e| Error::AddressError(e.to_string()))?
        .require_network(Network::Testnet)
        .map_err(|e| Error::NetworkError(e.to_string()))?;

    let pubkey = wallet.get_public_key().await?;
    let (xonly_pubkey, _parity) = pubkey.inner.x_only_public_key();
    // 构建atomicals payload
    let payload = PayloadWrapper {
        args: {
            let (time, nonce) = time_nonce();

            Payload {
                bitworkc: config.mint_bitworkc.clone().unwrap_or_else(|| "".to_string()),
                mint_ticker: config.tick.clone(),
                nonce,
                time,
            }
        },
    };
    let payload_encoded = cbor(&payload)?;
    
    // 准备commit-reveal配置
    let (script, script_address) = prepare_commit_reveal_config(
        "dmt",  // op_type for FT
        &xonly_pubkey,
        &payload_encoded,
        Network::Testnet
    ).await?;
    
    // 获取 UTXO 列表和网络费率
    let utxos = wallet.get_utxos().await?;
    let fee_rate = wallet.get_network_fee_rate().await?;
    
    // 计算reveal交易所需费用
    let reveal_size = tx_size::calculate_reveal_size(
        1, // 一个reveal input
        1, // 一个输出
        script.len(), // hash_lock script长度
    );
    let reveal_fee = Amount::from_sat((reveal_size * fee_rate) as u64);
    
    // 计算commit交易输出值（需要考虑reveal交易的输入和输出）
    let commit_output_value = reveal_fee + Amount::from_sat(config.mint_amount.0);
    
    // 计算commit交易本身的费用
    let commit_size = tx_size::calculate_commit_size(
        1, // 一个输入
        2, // 两个输出（P2TR输出和找零）
    );
    let commit_fee = Amount::from_sat((commit_size * fee_rate) as u64);
    
    // 选择合适的 UTXO 并计算手续费
    let (selected_utxos, _) = select_utxos(
        &utxos,
        commit_output_value + commit_fee,
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
    let change_amount = match total_input.checked_sub(commit_fee) {
        Some(remaining) => match remaining.checked_sub(commit_output_value) {
            Some(change) => change,
            None => return Err(Error::InvalidAmount("Not enough funds after fees".into())),
        },
        None => return Err(Error::InvalidAmount("Not enough funds to cover fees".into())),
    };

    // 创建 commit 交易输出
    let mut commit_outputs = vec![
        TxOut {
            value: commit_output_value,
            script_pubkey: script_address.script_pubkey(),
        },
    ];
    
    // 如果有找零，添加找零输出
    if change_amount > Amount::from_sat(546) {
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
        commit_psbt.inputs[i].tap_internal_key = Some(xonly_pubkey);
        // 修复 tap_key_origins 的类型
        let mut origins = std::collections::BTreeMap::new();
        origins.insert(xonly_pubkey, (vec![], (bitcoin::bip32::Fingerprint::default(), bitcoin::bip32::DerivationPath::default())));
        commit_psbt.inputs[i].tap_key_origins = origins;
    }
    
    log!("Created commit PSBT");

    // 创建 reveal 交易
    let reveal_input = TxIn {
        previous_output: OutPoint::new(commit_tx.txid(), 0),
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Default::default(),
    };

    let reveal_tx = Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![reveal_input],
        output: vec![
            TxOut {
                value: Amount::from_sat(config.mint_amount.0),
                script_pubkey: address.script_pubkey(),
            },
        ],
    };

    let mut reveal_psbt = Psbt::from_unsigned_tx(reveal_tx.clone())
        .map_err(|e| Error::PsbtError(format!("Failed to create reveal PSBT: {}", e)))?;

    // 添加reveal交易的witness_script和tap_internal_key
    reveal_psbt.inputs[0].witness_script = Some(script.clone());
    reveal_psbt.inputs[0].tap_internal_key = Some(xonly_pubkey);
    let mut origins = std::collections::BTreeMap::new();
    origins.insert(xonly_pubkey, (vec![], (bitcoin::bip32::Fingerprint::default(), bitcoin::bip32::DerivationPath::default())));
    reveal_psbt.inputs[0].tap_key_origins = origins;
    reveal_psbt.inputs[0].witness_utxo = Some(commit_tx.output[0].clone());
    
    // 添加 Taproot 特定字段
    let secp = Secp256k1::new();
    let mut builder = TaprootBuilder::new();
    builder = builder.add_leaf(0, script.clone())?;
    let merkle_root = builder.finalize(&secp, xonly_pubkey)?;
    
    // 获取 merkle_root，如果为 None 则返回错误
    let tap_merkle_root = merkle_root.merkle_root()
        .ok_or_else(|| Error::TransactionError("Failed to get merkle root".into()))?;
    reveal_psbt.inputs[0].tap_merkle_root = Some(tap_merkle_root);
    
    // 创建控制块
    let control_block = merkle_root.control_block(&(script.clone(), LeafVersion::TapScript))
        .ok_or_else(|| Error::TransactionError("Failed to create control block".into()))?;
    
    // 设置 tap_scripts
    let mut tap_scripts = std::collections::BTreeMap::new();
    tap_scripts.insert(control_block, (script.clone(), LeafVersion::TapScript));
    reveal_psbt.inputs[0].tap_scripts = tap_scripts;
    
    log!("Created reveal PSBT with Taproot script");

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
    
    // Broadcast commit transaction first
    let commit_txid = wallet.broadcast_transaction(commit_tx.clone()).await?;
    log!("Commit transaction broadcast successfully: {}", commit_txid);
    
    // Wait for a short time to ensure commit tx propagation
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                2000, // 2 seconds
            )
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    
    // Broadcast reveal transaction
    let reveal_txid = wallet.broadcast_transaction(reveal_tx.clone()).await?;
    log!("Reveal transaction broadcast successfully: {}", reveal_txid);
    
    let atomicals_tx = AtomicalsTx::new_with_commit_reveal(
        commit_tx,
        reveal_tx,
        Some(commit_txid),
        Some(reveal_txid),
    );

    Ok(atomicals_tx)
}
