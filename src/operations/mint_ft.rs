use crate::errors::Result;
use crate::operations::mining::{mine_transaction, MiningOptions};
use crate::types::{Arc20Config, Arc20Token, AtomicalsTx, BitworkInfo};
use crate::wallet::WalletProvider;
use bitcoin::{Script, Transaction, TxIn, TxOut};
use serde_json::json;

pub async fn mint_ft<W: WalletProvider>(
    wallet: &W,
    config: Arc20Config,
    mining_options: Option<MiningOptions>,
) -> Result<AtomicalsTx> {
    // Get wallet address
    let address = wallet.get_address().await?;

    // Create mint data
    let mint_data = json!({
        "p": "arc20",
        "op": "mint",
        "tick": config.tick,
        "amt": config.mint_amount.0,
    });

    // Create output script
    let script = Script::new_op_return(&serde_json::to_vec(&mint_data)?);
    
    // Create transaction template
    let mut tx = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn::default()], // Will be filled by wallet
        output: vec![
            TxOut {
                value: 0,
                script_pubkey: script,
            }
        ],
    };

    // If mining is required
    if let Some(bitwork) = config.mint_bitworkc.as_ref() {
        let bitwork_info = BitworkInfo::new(bitwork.clone());
        let options = mining_options.unwrap_or_default();
        
        // Mine the transaction
        let mining_result = mine_transaction(tx, bitwork_info, options)?;
        tx = mining_result.transaction;
    }

    // Sign and broadcast transaction
    let signed_tx = wallet.sign_transaction(tx, &[]).await?;
    let txid = wallet.broadcast_transaction(signed_tx).await?;

    // Create token instance
    let mut token = Arc20Token::new(config);
    token.add_holder(address, token.config.mint_amount)?;

    Ok(AtomicalsTx::new(tx, vec![]).with_atomicals_id(txid))
}
