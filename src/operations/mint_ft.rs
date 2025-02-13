use crate::errors::{Error, Result};
use crate::operations::mining::{mine_transaction, MiningOptions};
use crate::types::{Arc20Config, Arc20Token, AtomicalsTx, mint::BitworkInfo};
use crate::wallet::WalletProvider;
use bitcoin::{Transaction, TxIn, TxOut, ScriptBuf};
use bitcoin::opcodes::all::OP_RETURN;
use bitcoin::locktime::absolute::LockTime;
use serde::{Serialize, Deserialize};
use std::error::Error as StdError;
use web_sys::console;

#[derive(Debug, Serialize, Deserialize)]
struct MintData {
    p: String,
    op: String,
    tick: String,
    amt: u64,
}

pub async fn mint_ft<W: WalletProvider>(
    wallet: &W,
    config: Arc20Config,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    console::log_1(&"Starting mint_ft operation...".into());

    // Get wallet address
    console::log_1(&"Getting wallet address...".into());
    let address = wallet.get_address().await.map_err(|e| Error::WalletError(e.to_string()))?;
    console::log_1(&format!("Got wallet address: {}", address).into());
    
    // Create mint data
    let mint_data = MintData {
        p: "arc20".to_string(),
        op: "mint".to_string(),
        tick: config.tick.clone(),
        amt: config.mint_amount.0,
    };
    console::log_1(&format!("Created mint data: {:?}", mint_data).into());
    
    // Create data payload
    let payload = serde_json::to_vec(&mint_data).map_err(|e| Error::from(e))?;
    console::log_1(&format!("Created payload of size: {}", payload.len()).into());
    
    // Create OP_RETURN script
    let mut builder = bitcoin::script::Builder::new();
    builder = builder.push_opcode(OP_RETURN);
    
    // Split payload into chunks that fit within push limits
    for chunk in payload.chunks(32) {
        if chunk.len() <= 32 {
            let mut array = [0u8; 32];
            array[..chunk.len()].copy_from_slice(chunk);
            builder = builder.push_opcode(bitcoin::opcodes::all::OP_PUSHBYTES_32)
                .push_slice(&array);
        } else {
            return Err(Error::Generic(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Script chunk too large"
            ))));
        }
    }
    
    let script = builder.into_script();
    console::log_1(&"Created OP_RETURN script".into());
    
    // Create transaction template
    let mut tx = Transaction {
        version: 2,
        lock_time: LockTime::ZERO,
        input: vec![], // Empty inputs, will be filled by wallet
        output: vec![
            TxOut {
                value: 1000, // Minimum output value
                script_pubkey: script,
            }
        ],
    };
    console::log_1(&format!("Created transaction template: {:?}", tx).into());

    // If mining is required
    if let Some(bitwork) = config.mint_bitworkc.as_ref() {
        console::log_1(&format!("Mining required with bitwork: {}", bitwork).into());
        let bitwork_info = BitworkInfo::new(bitwork.clone());
        let options = mining_options.unwrap_or_default();
        
        // Mine the transaction
        console::log_1(&"Starting transaction mining...".into());
        let mining_result = mine_transaction(tx, bitwork_info, options)?;
        tx = mining_result.transaction;
        console::log_1(&"Mining completed".into());
    }

    // Sign the transaction with UTXOs
    console::log_1(&"Attempting to sign transaction...".into());
    let signed_tx = match wallet.sign_transaction(tx, &[]).await {
        Ok(signed) => {
            console::log_1(&"Transaction signed successfully".into());
            signed
        },
        Err(e) => {
            console::error_1(&format!("Failed to sign transaction: {}", e).into());
            return Err(Error::WalletError(format!("Failed to sign transaction: {}", e)));
        }
    };

    // Get the transaction ID
    let txid = signed_tx.txid().to_string();
    console::log_1(&format!("Generated transaction ID: {}", txid).into());

    // Create token instance
    let mut token = Arc20Token::new(config);
    token.add_holder(address, token.config.mint_amount)?;
    console::log_1(&"Token instance created and holder added".into());

    Ok(AtomicalsTx::new(signed_tx, vec![]).with_atomicals_id(txid))
}
