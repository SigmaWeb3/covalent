use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use rlp::{Decodable, Encodable, Rlp};
use sled::Db;

use crate::types::{Block, Hash, Header, SignedTransaction, U64};

const LATEST_HEADER_KEY: &[u8] = b"latest_block";
const BLOCK_TREE: &[u8] = b"block_tree";
const NUMBER_HASH_TREE: &[u8] = b"number_hash_tree";
const TX_TREE: &[u8] = b"transaction_tree";

#[async_trait]
pub trait Chain: Sync + Send {
    async fn save_block(&self, block: Block) -> Result<()>;

    async fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>>;

    async fn get_block_by_number(&self, number: &U64) -> Result<Option<Block>>;

    async fn get_latest_block(&self) -> Result<Header>;

    async fn get_tx_by_hash(&self, hash: &Hash) -> Result<Option<SignedTransaction>>;
}

pub struct CovalentChain {
    db: Arc<Db>,
}

#[async_trait]
impl Chain for CovalentChain {
    async fn save_block(&self, block: Block) -> Result<()> {
        let block_t = self.db.open_tree(BLOCK_TREE)?;
        block_t.insert(block.header_hash(), block.rlp_bytes().to_vec())?;
        block_t.insert(LATEST_HEADER_KEY, block.header.rlp_bytes().to_vec())?;

        self.db.open_tree(NUMBER_HASH_TREE)?.insert(
            u64_le_bytes(&block.header.number),
            block.header_hash().0.to_vec(),
        )?;

        let tx_t = self.db.open_tree(TX_TREE)?;
        for tx in block.txs.iter() {
            tx_t.insert(tx.tx_hash, tx.rlp_bytes().to_vec())?;
        }

        Ok(())
    }

    async fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        match self.db.open_tree(BLOCK_TREE)?.get(hash)? {
            None => Ok(None),
            Some(raw) => Ok(Some(Block::decode(&Rlp::new(raw.as_ref()))?)),
        }
    }

    async fn get_block_by_number(&self, number: &U64) -> Result<Option<Block>> {
        if let Some(raw) = self
            .db
            .open_tree(NUMBER_HASH_TREE)?
            .get(u64_le_bytes(number))?
        {
            return self.get_block_by_hash(&Hash::from_slice(&raw)).await;
        }

        Ok(None)
    }

    async fn get_latest_block(&self) -> Result<Header> {
        let raw = self
            .db
            .open_tree(BLOCK_TREE)?
            .get(LATEST_HEADER_KEY)?
            .unwrap();
        let ret = Header::decode(&Rlp::new(raw.as_ref()))?;
        Ok(ret)
    }

    async fn get_tx_by_hash(&self, hash: &Hash) -> Result<Option<SignedTransaction>> {
        match self.db.open_tree(TX_TREE)?.get(hash)? {
            None => Ok(None),
            Some(raw) => Ok(Some(SignedTransaction::decode(&Rlp::new(raw.as_ref()))?)),
        }
    }
}

impl CovalentChain {
    pub fn new(path: PathBuf) -> Self {
        CovalentChain {
            db: Arc::new(sled::open(path).unwrap()),
        }
    }
}

fn u64_le_bytes(input: &U64) -> Vec<u8> {
    let mut buf = [0u8; 8];
    input.to_little_endian(&mut buf);
    buf.to_vec()
}
