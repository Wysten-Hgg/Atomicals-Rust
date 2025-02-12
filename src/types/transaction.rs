use bitcoin::{Transaction, TxOut};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicalsTx {
    pub raw_tx: Transaction,
    pub inputs: Vec<TxOut>,
    pub atomicals_id: Option<String>,
}

impl AtomicalsTx {
    pub fn new(raw_tx: Transaction, inputs: Vec<TxOut>) -> Self {
        Self {
            raw_tx,
            inputs,
            atomicals_id: None,
        }
    }

    pub fn with_atomicals_id(mut self, id: String) -> Self {
        self.atomicals_id = Some(id);
        self
    }

    pub fn txid(&self) -> String {
        self.raw_tx.txid().to_string()
    }
}
