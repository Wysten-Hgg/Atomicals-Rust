use std::str::FromStr;
use crate::errors::{Error, Result};
use crate::operations::mining::{mine_transaction, MiningOptions};
use crate::types::{Arc20Config, AtomicalsTx, mint::BitworkInfo, AtomicalsPayload};
use crate::utils::script_builder::build_atomicals_op_return;
use crate::wallet::WalletProvider;
use bitcoin::{Transaction, TxIn, TxOut, ScriptBuf, Network};
use bitcoin::address::{NetworkUnchecked, Address};
use bitcoin::locktime::absolute::LockTime;
use web_sys::console;

pub async fn mint_ft<W: WalletProvider>(
    wallet: &W,
    config: Arc20Config,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    console::log_1(&"Starting mint_ft operation...".into());

    // Get wallet address
    console::log_1(&"Getting wallet address...".into());
    let address_str = wallet.get_address().await.map_err(|e| Error::WalletError(e.to_string()))?;
    let unchecked_address = Address::<NetworkUnchecked>::from_str(&address_str)
        .map_err(|e| Error::WalletError(format!("Invalid address format: {}", e)))?;
    
    // 默认使用比特币主网，因为我们不再进行网络验证
    let address = unchecked_address.require_network(Network::Testnet)
        .map_err(|e| Error::WalletError(format!("Invalid network: {}", e)))?;
    
    console::log_1(&format!("Got wallet address: {:?}", address).into());
    
    // Create Atomicals payload for minting existing FT
    let payload = AtomicalsPayload::new_mint_ft(config.tick.clone());
    
    // Create OP_RETURN output
    let op_return_script = build_atomicals_op_return(&payload)?;
    
    // Get public key for deriving script_pubkey
    let _public_key = wallet.get_public_key().await.map_err(|e| Error::WalletError(e.to_string()))?;
    
    // Create transaction outputs
    let outputs = vec![
        // OP_RETURN output
        TxOut {
            value: 0,
            script_pubkey: op_return_script,
        },
        // Mint amount output to recipient
        TxOut {
            value: config.mint_amount.0,
            script_pubkey: address.script_pubkey(),
        }
    ];

    // Create unsigned transaction
    let mut tx = Transaction {
        version: 2,
        lock_time: LockTime::ZERO,
        input: vec![], // Empty inputs, wallet will handle input selection
        output: outputs.clone(),
    };

    // Sign transaction (wallet handles input selection)
    tx = wallet.sign_transaction(tx, &outputs).await.map_err(|e| Error::WalletError(e.to_string()))?;

    // Mine transaction if required
    if let Some(mining_opts) = mining_options {
        // Create bitwork info from mining options
        let bitwork = BitworkInfo {
            prefix: "0000".to_string(),
            ext: None,
            difficulty: mining_opts.target_tx_size as u32,
        };
        
        let mining_result = mine_transaction(tx, bitwork, mining_opts)?;
        tx = mining_result.transaction;
    }

    Ok(AtomicalsTx::new(tx, outputs))
}
