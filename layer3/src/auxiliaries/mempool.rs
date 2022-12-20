use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use primitive_types::H160;

use crate::types::{Block, SignedTransaction};

pub trait MemPool {
    fn push_transaction(&self, tx: SignedTransaction) -> Result<()>;
    fn package_transactions(&self) -> Result<Vec<SignedTransaction>>;
    fn reset(&self, block: &Block) -> Result<()>;
}

#[derive(Clone)]
pub struct ChannelMap {
    map: Arc<RwLock<HashMap<H160, Vec<SignedTransaction>>>>,
}

impl MemPool for ChannelMap {
    fn push_transaction(&self, tx: SignedTransaction) -> Result<()> {
        let mut map = self.map.write().unwrap();
        map.entry(tx.from)
            .and_modify(|txs| txs.push(tx.clone()))
            .or_insert(vec![tx]);
        Ok(())
    }

    fn package_transactions(&self) -> Result<Vec<SignedTransaction>> {
        let mut packaged = Vec::with_capacity(200);

        let map = self.map.read().unwrap();
        for txs in map.values() {
            if packaged.len() >= packaged.capacity() {
                return Ok(packaged);
            }
            if txs.is_empty() {
                continue;
            }

            packaged.push(txs.first().cloned().unwrap());
        }

        Ok(packaged)
    }

    fn reset(&self, block: &Block) -> Result<()> {
        let mut map = self.map.write().unwrap();

        for block_tx in &block.txs {
            let txs = match map.get_mut(&block_tx.from) {
                Some(txs) => txs,
                None => continue,
            };
            if txs.is_empty() {
                drop(txs);
                map.remove(&block_tx.from);

                continue;
            }

            let idx = match txs.iter_mut().position(|tx| tx.hash == block_tx.hash) {
                Some(idx) => idx,
                None => continue,
            };
            if idx == txs.len() - 1 {
                drop(txs);
                map.remove(&block_tx.from);

                continue;
            }

            *txs = txs.split_off(idx + 1);
        }

        Ok(())
    }
}
