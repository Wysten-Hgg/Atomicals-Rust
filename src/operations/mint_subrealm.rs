use crate::types::{AtomicalsTx, subrealm::{SubrealmConfig, SubrealmClaimType, SubrealmRule, RuleOutput}};
use crate::types::mint::{BitworkInfo, MintConfig, MintResult};
use crate::errors::{Error, Result};
use crate::wallet::{WalletProvider, Utxo};
use crate::types::wasm::{WasmTransaction, WasmBitworkInfo};
use crate::operations::mining::{mine_transaction, MiningOptions, MiningResult};
use crate::utils::tx_size::{self, ScriptType};
use crate::utils::script::{append_mint_update_reveal_script, time_nonce, cbor};

use bitcoin::{
    Amount, Network, Transaction, TxIn, TxOut, Sequence,
    psbt::Psbt, ScriptBuf, Address, taproot::{TapTree, TaprootBuilder, LeafVersion},
    transaction::Version, key::XOnlyPublicKey, secp256k1::Secp256k1,
    OutPoint,
};
use bitcoin::opcodes::all::OP_RETURN;
use bitcoin::hashes::{sha256, Hash};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;
use serde_cbor;
use serde::{Serialize, Deserialize};
use web_sys;
use js_sys;
use wasm_bindgen_futures;
use regex::Regex;
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadWrapper {
    pub args: Payload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitworkc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitworkr: Option<String>,
    pub request_subrealm: String,
    pub parent_realm: String,
    pub claim_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<serde_json::Value>,
}

/// 准备 commit-reveal 配置
async fn prepare_commit_reveal_config(
    op_type: &str,
    child_node_xonly_pubkey: &XOnlyPublicKey,
    atomicals_payload: &[u8],
    network: Network
) -> Result<(ScriptBuf, Address)> {
    // 构建 Taproot 脚本
    let script = append_mint_update_reveal_script(op_type, &child_node_xonly_pubkey, atomicals_payload)?;
    log!("Taproot script: {:?}", script.clone());
                    
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
    let script_address = Address::from_script(&tr_script, network)?;
    
    Ok((script, script_address))
}

/// 铸造 Subrealm
pub async fn mint_subrealm<W: WalletProvider>(
    wallet: &W,
    mut config: SubrealmConfig,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    log!("Starting mint_subrealm operation...");

    // 验证 Subrealm 名称
    if let Err(e) = config.validate_name() {
        return Err(Error::RealmNameInvalid(e.to_string()));
    }

    // 分割 Subrealm 名称获取最后一部分
    let parts: Vec<&str> = config.name.split('.').collect();
    let subrealm_part = parts[parts.len() - 1];

    // 获取钱包公钥和地址
    let address_str = wallet.get_address().await?;
    let address = Address::from_str(&address_str)
        .map_err(|e| Error::AddressError(e.to_string()))?
        .require_network(Network::Testnet)
        .map_err(|e| Error::NetworkError(e.to_string()))?;

    let pubkey = wallet.get_public_key().await?;
    let (xonly_pubkey, _parity) = pubkey.inner.x_only_public_key();

    // 获取父 Realm 的 UTXO 并验证所有权
    let parent_info = wallet.get_atomical_by_id(&config.parent_realm_id).await?;
    let parent_location = parent_info.get_current_location()
        .ok_or_else(|| Error::AtomicalNotFound(format!("Parent realm {} not found", config.parent_realm_id)))?;
    let parent_script = ScriptBuf::from_hex(&parent_location.script)
        .map_err(|e| Error::ScriptError(format!("Failed to parse parent realm script: {}", e)))?;
    
    let parent_outpoint = OutPoint::from_str(&parent_location.location)
        .map_err(|e| Error::InvalidInput(format!("Invalid parent location: {}", e)))?;

    log!("Parent location info - location: {}, txid: {}, index: {}", 
        parent_location.location,
        parent_location.txid,
        parent_location.index
    );

    // 验证 location 格式
    if !parent_location.location.contains(':') {
        return Err(Error::InvalidInput(format!("Invalid parent location: {}", parent_location.location)));
    }
    let location_parts: Vec<&str> = parent_location.location.split(':').collect();
    if location_parts.len() != 2 {
        return Err(Error::InvalidInput("Invalid parent location: OutPoint not in <txid>:<vout> format".into()));
    }
    
    let script_pubkey = address.script_pubkey();
    
    // 计算 scripthash (Electrum 格式)
    let script_bytes = script_pubkey.as_bytes();
    let hash = sha256::Hash::hash(&sha256::Hash::hash(script_bytes).to_byte_array());
    
    // 反转字节序并转换为十六进制字符串
    let scripthash = hash.to_byte_array()
        .iter()
        .rev()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    // 验证父 Realm 所有权
    log!("parent_location.scripthash: {} current scripthash {})", 
    parent_location.scripthash, 
    scripthash
    );
    if config.claim_type == SubrealmClaimType::Direct && parent_location.scripthash != scripthash {
        return Err(Error::OwnershipError("Parent realm not owned by current wallet".into()));
    }

    // 获取父 Realm 的 subrealm 规则
    let subrealms = match &parent_info.state {
        Some(state) => state.latest.as_ref()
            .and_then(|latest| latest.subrealms.as_ref())
            .ok_or_else(|| Error::InvalidInput("Parent realm has no subrealm rules".into()))?,
        None => return Err(Error::InvalidInput("Parent realm has no state".into())),
    };

    // 检查规则数组是否为空
    if subrealms.rules.is_empty() {
        return Err(Error::InvalidInput("Parent realm has no rules".into()));
    }

    // 根据铸造类型选择规则
    let matched_rule = if config.claim_type == SubrealmClaimType::Rule {
        let mut matched = None;
        for rule in &subrealms.rules {
            if let Ok(re) = Regex::new(&format!("^{}$", &rule.p)) {
                if re.is_match(&subrealm_part.to_string().clone()) {
                    // 验证规则中的价格范围
                    if let Some(outputs) = rule.o.as_object() {
                        for (_, output) in outputs {
                            if let Some(price) = output.get("v").and_then(|v| v.as_u64()) {
                                if price == 0 || price > 100000000 {
                                    return Err(Error::InvalidInput(format!(
                                        "Invalid price in rule: {} sats. Must be between 1 and 100000000 sats",
                                        price
                                    )));
                                }
                            }
                        }
                    }
                    matched = Some(rule);
                    break;
                }
            }
        }
        matched.ok_or_else(|| Error::InvalidInput("No matching rule found".into()))?
    } else {
        // 直接铸造使用第一条规则
        &subrealms.rules[0]
    };

    // 设置工作量证明要求
    if config.claim_type == SubrealmClaimType::Rule {
        if let Some(bitworkc) = &matched_rule.bitworkc {
            config.bitworkc = Some(bitworkc.clone());
        }
    }

    // 构建 atomicals payload
    let payload = PayloadWrapper {
        args: {
            let mut payload = Payload {
                request_subrealm: subrealm_part.to_string(),
                parent_realm: config.parent_realm_id.clone(),
                claim_type: config.claim_type.as_str().to_string(),
                nonce: None,
                time: None,
                bitworkc: None,
                bitworkr: None,
                container: config.container.clone(),
                meta: config.meta.clone(),
                ctx: config.ctx.clone(),
                init: config.init.clone(),
            };
            
            // 对于非直接铸造类型，添加额外字段
            if config.claim_type != SubrealmClaimType::Direct {
                let (time, nonce) = time_nonce();
                payload.time = Some(time);
                payload.nonce = Some(nonce);
            }
            
            // 添加可选的 bitwork 字段
            if let Some(bitworkc) = &config.bitworkc {
                payload.bitworkc = Some(bitworkc.clone());
            }
            
            if let Some(bitworkr) = &config.bitworkr {
                payload.bitworkr = Some(bitworkr.to_string());
            }
            
            payload
        },
    };
    
    // 序列化 payload 为 CBOR 格式
    let atomicals_payload = cbor(&payload)?;
    
    // 准备 commit-reveal 配置
    let (script, script_address) = prepare_commit_reveal_config(
        "nft",  // op_type for Subrealm (same as Realm)
        &xonly_pubkey,
        &atomicals_payload,
        Network::Testnet
    ).await?;
    
    // 获取 UTXO 列表和网络费率
    let utxos = wallet.get_utxos().await?;
    if utxos.is_empty() {
        return Err(Error::InvalidAmount("No UTXOs available".into()));
    }
    
    // 获取网络费率
    let fee_rate = wallet.get_network_fee_rate().await?;
    
    // 计算reveal交易所需费用（现在包括两个输入和一个输出）
    let reveal_size = tx_size::calculate_reveal_size(
        2, // 两个输入：commit输出和父realm
        3, // 一个合并的输出
        script.len(), // hash_lock script长度
    );
    let reveal_fee = if config.bitworkr.is_some() {
        Amount::from_sat((reveal_size * fee_rate * 1.2) as u64) // 增加 20% 的手续费
    } else {
        Amount::from_sat((reveal_size * fee_rate) as u64)
    };
    
    log!("Calculated reveal fee: {} sats (BitworkR: {})", 
        reveal_fee.to_sat(), 
        config.bitworkr.is_some()
    );
    
    // 计算commit交易输出值（需要考虑reveal交易的输入和输出）
    let commit_output_value = reveal_fee + Amount::from_sat(config.sats_output + parent_location.value);
    
    // 计算commit交易本身的费用
    let commit_size = tx_size::calculate_commit_size(
        1, // 一个输入（资金输入）
        2, // 两个输出（P2TR输出和找零）
    );
    let commit_fee = Amount::from_sat((commit_size * fee_rate) as u64);
    
    // 选择合适的 UTXO 并计算手续费
    let (selected_utxos, _) = select_utxos(
        &utxos,
        commit_output_value + commit_fee,
        fee_rate,
        0
    )?;
    
    // 创建交易输入
    let mut inputs = vec![
        // 添加其他资金输入
        TxIn {
            previous_output: selected_utxos[0].outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Default::default(),
        },
    ];

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

    // 创建 commit 交易输出，确保正确的顺序
    let mut commit_outputs = vec![
        // 第一个输出是 subrealm 的位置
        TxOut {
            value: commit_output_value,
            script_pubkey: script_address.script_pubkey(),
        },
    ];

    // 如果有找零，添加到最后
    if change_amount > Amount::from_sat(546) {
        commit_outputs.push(TxOut {
            value: change_amount,
            script_pubkey: address.script_pubkey(),
        });
    }

    let commit_tx = Transaction {
        version: Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: inputs,
        output: commit_outputs,
    };

    let mut commit_psbt = Psbt::from_unsigned_tx(commit_tx.clone())
        .map_err(|e| Error::PsbtError(format!("Failed to create commit PSBT: {}", e)))?;
        
    // 添加其他输入的 UTXO 信息到 PSBT
    for (i, utxo) in selected_utxos.iter().enumerate() {
        let psbt_index = i; // 因为没有父 Realm 输入
        commit_psbt.inputs[psbt_index].witness_utxo = Some(utxo.txout.clone());
        commit_psbt.inputs[psbt_index].tap_internal_key = Some(xonly_pubkey);
        let mut origins = std::collections::BTreeMap::new();
        origins.insert(xonly_pubkey, (vec![], (bitcoin::bip32::Fingerprint::default(), bitcoin::bip32::DerivationPath::default())));
        commit_psbt.inputs[psbt_index].tap_key_origins = origins.clone();
    }
    
    log!("Created commit PSBT");

    // Mine transactions if needed
    let mut mined_commit_tx = commit_tx.clone();
    if let Some(ref mining_opts) = mining_options {
        if let Some(ref bitworkc) = config.bitworkc {
            log!("Mining commit transaction...");
            let mining_result = mine_transaction(
                WasmTransaction::from_transaction(&commit_tx),
                WasmBitworkInfo::from_bitwork_info(&BitworkInfo::new(bitworkc.clone())),
                MiningOptions::new(),
            ).await?;

            let mining_result: MiningResult = serde_wasm_bindgen::from_value(mining_result)?;
            if mining_result.success {
                if let Some(mined_tx) = mining_result.get_transaction() {
                    log!("Mining successful, transaction sequence: {:?}", mined_tx.input[0].sequence);
                    
                    // 创建新的PSBT，保留所有原始信息
                    commit_psbt = Psbt::from_unsigned_tx(mined_tx.clone())
                        .map_err(|e| Error::PsbtError(format!("Failed to create PSBT after mining commit tx: {}", e)))?;
                    
                    // 重新添加UTXO信息
                    for (i, utxo) in selected_utxos.iter().enumerate() {
                        commit_psbt.inputs[i].witness_utxo = Some(utxo.txout.clone());
                        commit_psbt.inputs[i].tap_internal_key = Some(xonly_pubkey);
                        let mut origins = std::collections::BTreeMap::new();
                        origins.insert(xonly_pubkey, (vec![], (bitcoin::bip32::Fingerprint::default(), bitcoin::bip32::DerivationPath::default())));
                        commit_psbt.inputs[i].tap_key_origins = origins;
                        
                        // 确保PSBT中的sequence值与挖矿结果一致
                        if i == 0 {
                            commit_psbt.unsigned_tx.input[i].sequence = mined_tx.input[0].sequence;
                        }
                    }
                    
                    log!("PSBT updated with mined transaction and UTXO info");
                    log!("PSBT sequence value: {:?}", commit_psbt.unsigned_tx.input[0].sequence);
                    mined_commit_tx = mined_tx.clone();
                }
            } else {
                return Err(Error::MiningError("Failed to mine commit transaction".into()));
            }
        }
    }

    // 创建 reveal 交易
    let mut reveal_inputs = vec![
        TxIn {  // commit 输出放在第一位
            previous_output: OutPoint::new(mined_commit_tx.txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Default::default(),
        },
    ];
    if config.claim_type == SubrealmClaimType::Direct {
        reveal_inputs.push(TxIn {  // 父 Realm 输入放在第二位
            previous_output: parent_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Default::default(),
        });
    }

    log!("Subrealm Amount: {:?}", Amount::from_sat(config.sats_output.clone()));
    log!("Realm Amount: {:?}", Amount::from_sat(parent_location.value.clone()));
    let mut reveal_outputs = vec![
        TxOut {  // Subrealm 输出放在第一位
            value: Amount::from_sat(config.sats_output),
            script_pubkey: address.script_pubkey().clone(),
        },
    ];
    if config.claim_type == SubrealmClaimType::Direct {
        reveal_outputs.push(TxOut {  // 父 Realm 返回放在第二位
            value: Amount::from_sat(parent_location.value),
            script_pubkey: parent_script.clone(),
        });
    }
  
    let reveal_tx = Transaction {
        version: Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: reveal_inputs,
        output: reveal_outputs,
    };

    let mut reveal_psbt = Psbt::from_unsigned_tx(reveal_tx.clone())
        .map_err(|e| Error::PsbtError(format!("Failed to create reveal PSBT: {}", e)))?;

   

    // 第一个输入保持不变
    reveal_psbt.inputs[0].witness_script = Some(script.clone());
    reveal_psbt.inputs[0].tap_internal_key = Some(xonly_pubkey);
    reveal_psbt.inputs[0].witness_utxo = Some(mined_commit_tx.output[0].clone());

    if config.claim_type == SubrealmClaimType::Direct {
         // 从父Realm的script中提取公钥
        let parent_internal_key = {
            // 移除OP_1前缀(0x51)和PUSHBYTES_32前缀(0x20)
            let pubkey_hex = parent_script.as_bytes()
                .get(2..)
                .ok_or_else(|| Error::TransactionError("Invalid parent script length".into()))?;
            XOnlyPublicKey::from_slice(pubkey_hex)
                .map_err(|e| Error::TransactionError(format!("Failed to parse parent internal key: {}", e)))?
        };
        // 第二个输入使用父Realm的公钥
        reveal_psbt.inputs[1].witness_utxo = Some(TxOut {
            value: Amount::from_sat(parent_location.value),
            script_pubkey: parent_script.clone(),
        });
        reveal_psbt.inputs[1].tap_internal_key = Some(parent_internal_key);
    }
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
    tap_scripts.insert(control_block.clone(), (script.clone(), LeafVersion::TapScript));
    reveal_psbt.inputs[0].tap_scripts = tap_scripts;

    log!("Created reveal PSBT with Taproot script");

    // Mine transactions if needed
    if let Some(ref mining_opts) = mining_options {
        if let Some(ref bitworkr) = config.bitworkr {
            log!("Mining reveal transaction...");
            let mining_result = mine_transaction(
                WasmTransaction::from_transaction(&reveal_tx),
                WasmBitworkInfo::from_bitwork_info(&BitworkInfo::new(bitworkr.clone())),
                MiningOptions::new(),
            ).await?;

            let mining_result: MiningResult = serde_wasm_bindgen::from_value(mining_result)?;
            if mining_result.success {
                if let Some(mined_tx) = mining_result.get_transaction() {
                    let mut updated_reveal_tx = reveal_tx.clone();
                
                    // 只从挖矿结果中获取nonce(sequence)字段
                    updated_reveal_tx.input[0].sequence = mined_tx.input[0].sequence;
                    
                    // 重建PSBT，使用更新的交易但保留原始属性
                    let mut new_reveal_psbt = Psbt::from_unsigned_tx(updated_reveal_tx)
                        .map_err(|e| Error::PsbtError(format!("Failed to create updated PSBT: {}", e)))?;
                    
                    // 手动保留所有重要的PSBT输入信息
                    new_reveal_psbt.inputs[0].witness_utxo = Some(mined_commit_tx.output[0].clone());
                    new_reveal_psbt.inputs[0].witness_script = Some(script.clone());
                    new_reveal_psbt.inputs[0].tap_internal_key = Some(xonly_pubkey);
                    new_reveal_psbt.inputs[0].tap_merkle_root = Some(tap_merkle_root);
                    
                    // 设置 tap_scripts
                    let mut tap_scripts = std::collections::BTreeMap::new();
                    tap_scripts.insert(control_block.clone(), (script.clone(), LeafVersion::TapScript));
                    new_reveal_psbt.inputs[0].tap_scripts = tap_scripts;
                    
                    // 更新PSBT
                    reveal_psbt = new_reveal_psbt;
                    
                    // 详细记录PSBT状态用于调试
                    log!("Reveal PSBT after mining - input count: {}", reveal_psbt.inputs.len());
                    log!("PSBT input sequence: {:?}", reveal_psbt.unsigned_tx.input[0].sequence);
                    log!("PSBT has witness_utxo: {}", reveal_psbt.inputs[0].witness_utxo.is_some());
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

fn select_utxos(utxos: &[Utxo], target_amount: Amount, fee_rate: f64, additional_outputs: usize) -> Result<(Vec<Utxo>, Amount)> {
    // 预计输出的脚本类型
    let mut output_types = vec![
        ScriptType::P2WPKH, // commit tx 的第一个输出
        ScriptType::P2WPKH, // commit tx 的第二个输出
    ];
    
    // 添加额外输出的脚本类型
    for _ in 0..additional_outputs {
        output_types.push(ScriptType::P2WPKH);
    }
    
    // 找到第一个满足条件的UTXO
    for utxo in utxos {
        let mut selected_utxos = vec![utxo.clone()];
        
        // 获取输入的脚本类型
        let script_type = ScriptType::from_script(&utxo.txout.script_pubkey)
            .ok_or_else(|| Error::TransactionError("Unsupported script type".into()))?;
            
        // 构建输入类型列表
        let input_types = vec![script_type];
            
        // 计算当前交易大小
        let tx_size = tx_size::calculate_tx_size(
            &input_types,
            &output_types,
            true  // 有 OP_RETURN 输出
        );
        
        // 计算预估手续费
        let fee = Amount::from_sat((tx_size.total_vsize as f64 * fee_rate) as u64);
        
        // 检查单个UTXO是否满足金额要求
        if let Some(remaining) = utxo.txout.value.checked_sub(fee) {
            if remaining >= target_amount {
                return Ok((selected_utxos, fee));
            }
        }
    }
    
    Err(Error::InvalidAmount("No single UTXO with sufficient funds found".into()))
}
