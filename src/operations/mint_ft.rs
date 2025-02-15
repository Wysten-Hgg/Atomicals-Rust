use crate::operations::mining::{mine_transaction, MiningOptions};
use crate::types::{AtomicalsTx, Arc20Config, mint::BitworkInfo};
use crate::errors::{Error, Result};
use crate::wallet::WalletProvider;
use bitcoin::{Amount, Network, Transaction, TxIn, TxOut, Txid};
use bitcoin::transaction::Version;
use bitcoin::address::Address;
use bitcoin::psbt::Psbt;
use std::str::FromStr;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (log::info!($($t)*))
}

pub async fn mint_ft<W: WalletProvider>(
    wallet: &W,
    config: Arc20Config,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    log!("Starting mint_ft operation...");

    // Get wallet address
    let address_str = wallet.get_address().await?;
    let address = Address::from_str(&address_str)
        .map_err(|e| Error::AddressError(e.to_string()))?
        .require_network(Network::Bitcoin)
        .map_err(|e| Error::NetworkError(e.to_string()))?;

    // Create commit transaction
    let commit_tx = Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![],
        output: vec![
            TxOut {
                value: Amount::from_sat(0),
                script_pubkey: address.script_pubkey(),
            },
            TxOut {
                value: Amount::from_sat(config.mint_amount.0),
                script_pubkey: address.script_pubkey(),
            },
        ],
    };

    let mut commit_psbt = Psbt::from_unsigned_tx(commit_tx)
        .map_err(|e| Error::PsbtError(format!("Failed to create commit PSBT: {}", e)))?;
    log!("Created commit PSBT");

    // Create reveal transaction
    let reveal_tx = Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![],
        output: vec![
            TxOut {
                value: Amount::from_sat(config.mint_amount.0),
                script_pubkey: address.script_pubkey(),
            },
        ],
    };

    let mut reveal_psbt = Psbt::from_unsigned_tx(reveal_tx)
        .map_err(|e| Error::PsbtError(format!("Failed to create reveal PSBT: {}", e)))?;
    log!("Created reveal PSBT");

    // Mine transactions if needed
    if let Some(ref mining_opts) = mining_options {
        if let Some(ref bitworkc) = config.mint_bitworkc {
            log!("Mining commit transaction...");
            let commit_tx = commit_psbt.extract_tx()
                .map_err(|e| Error::TransactionError(format!("Failed to extract commit tx for mining: {}", e)))?;
            let mining_result = mine_transaction(
                commit_tx,
                BitworkInfo::new(bitworkc.clone()),
                mining_opts.clone(),
            ).await?;
            
            if let Some(mined_tx) = mining_result.tx {
                commit_psbt = Psbt::from_unsigned_tx(mined_tx)
                    .map_err(|e| Error::PsbtError(format!("Failed to create PSBT after mining commit tx: {}", e)))?;
                log!("Commit transaction mined successfully");
            } else {
                return Err(Error::MiningError("Failed to mine commit transaction".into()));
            }
        }

        if let Some(ref bitworkr) = config.mint_bitworkr {
            log!("Mining reveal transaction...");
            let reveal_tx = reveal_psbt.extract_tx()
                .map_err(|e| Error::TransactionError(format!("Failed to extract reveal tx for mining: {}", e)))?;
            let mining_result = mine_transaction(
                reveal_tx,
                BitworkInfo::new(bitworkr.clone()),
                mining_opts.clone(),
            ).await?;
            
            if let Some(mined_tx) = mining_result.tx {
                reveal_psbt = Psbt::from_unsigned_tx(mined_tx)
                    .map_err(|e| Error::PsbtError(format!("Failed to create PSBT after mining reveal tx: {}", e)))?;
                log!("Reveal transaction mined successfully");
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
