use std::collections::BTreeMap;

use crate::types::Transaction;

#[derive(Debug, Default)]
pub struct Mempool {
    pub txs: BTreeMap<String, Transaction>,
}

impl Mempool {
    pub fn insert(&mut self, txid: String, tx: Transaction) {
        self.txs.insert(txid, tx);
    }

    pub fn snapshot(&self) -> Vec<Transaction> {
        self.txs.values().cloned().collect()
    }
}
