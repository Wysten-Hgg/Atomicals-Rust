use bitcoin::{Transaction, TxOut};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicalsTx {
    pub raw_tx: Transaction,
    pub inputs: Vec<TxOut>,
    pub atomicals_id: Option<String>,
    pub commit_tx: Option<Transaction>,
    pub reveal_tx: Option<Transaction>,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
}

impl AtomicalsTx {
    pub fn new(raw_tx: Transaction, inputs: Vec<TxOut>) -> Self {
        Self {
            raw_tx,
            inputs,
            atomicals_id: None,
            commit_tx: None,
            reveal_tx: None,
            commit_txid: None,
            reveal_txid: None,
        }
    }

    pub fn with_atomicals_id(mut self, atomicals_id: String) -> Self {
        self.atomicals_id = Some(atomicals_id);
        self
    }

    pub fn txid(&self) -> String {
        self.raw_tx.txid().to_string()
    }

    pub fn new_with_commit_reveal(
        commit_tx: Transaction,
        reveal_tx: Transaction,
        commit_txid: Option<String>,
        reveal_txid: Option<String>,
    ) -> Self {
        Self {
            raw_tx: reveal_tx.clone(),
            inputs: Vec::new(),
            atomicals_id: None,
            commit_tx: Some(commit_tx),
            reveal_tx: Some(reveal_tx),
            commit_txid,
            reveal_txid,
        }
    }
}
