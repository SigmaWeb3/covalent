use anyhow::Result;
use blake2b_ref::Blake2bBuilder;
use primitive_types::H256;
use serde::Serialize;

use crate::{
    consensus::ConsensusReceipt,
    types::{Block, NumberHash, SignedTransaction, TransactionReceipt},
};

use super::{
    common::{H256Ext, Hash},
    smt::{Error as SMTError, SMT},
    store::Store,
};

pub const TIP_BLOCK: &str = "TIP_BLOCK";
// prefix
pub const BLOCK_HASH: &str = "BLOCK_HASH";
pub const BLOCK: &str = "BLOCK";
pub const TRANSACTION: &str = "TRANSACTION";
pub const TRANSACTION_RECEIPT: &str = "TRANSACTION_RECEIPT";

pub trait Chain {
    fn tip_block(&self) -> Result<Block>;
    fn get_block_hash(&self, block_number: u64) -> Result<Option<H256>>;
    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>>;

    fn get_transaction(&self, tx_hash: H256) -> Result<Option<SignedTransaction>>;
    fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<TransactionReceipt>>;

    fn set_tip_block(&self, block_hash: H256) -> Result<()>;
    fn insert_block(&self, block: Block) -> Result<()>;
    fn insert_transaction(&self, tx: SignedTransaction) -> Result<()>;
    fn insert_transaction_receipt(&self, tx_hash: H256, receipt: TransactionReceipt) -> Result<()>;

    fn get_block(&self, number_hash: NumberHash) -> Result<Option<Block>> {
        match number_hash {
            NumberHash::Hash(block_hash) => self.get_block_by_hash(block_hash),
            NumberHash::Number(block_number) => self
                .get_block_hash(block_number)?
                .map(|bh| self.get_block_by_hash(bh).transpose())
                .flatten()
                .transpose(),
        }
    }

    fn insert_transactions(&self, txs: Vec<SignedTransaction>) -> Result<()> {
        for tx in txs {
            self.insert_transaction(tx)?;
        }
        Ok(())
    }

    fn insert_transaction_receipts(
        &self,
        tx_receipts: Vec<(H256, TransactionReceipt)>,
    ) -> Result<()> {
        for (tx_hash, receipt) in tx_receipts {
            self.insert_transaction_receipt(tx_hash, receipt).unwrap();
        }
        Ok(())
    }
}

pub struct ChannelChain {
    store: Store,
}

impl ChannelChain {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn apply_consensus_receipt(&self, receipt: &ConsensusReceipt) -> Result<()> {
        self.insert_block(receipt.block.clone())?;
        self.set_tip_block(receipt.block.block_hash())?;
        self.insert_transactions(receipt.block.txs.clone())?;
        self.insert_transaction_receipts(receipt.tx_receipts())?;

        let mut smt = SMT::new_with_store(self.store.clone()).map_err(SMTError)?;
        for (id, updated_channel) in &receipt.updated_channels {
            smt.update(id.to_h256(), updated_channel.clone())
                .map_err(SMTError)?;
        }

        Ok(())
    }
}

impl Chain for ChannelChain {
    fn tip_block(&self) -> Result<Block> {
        let block_hash: H256 = self.store.get(&TIP_BLOCK)?.unwrap();
        Ok(self.get_block_by_hash(block_hash)?.unwrap())
    }

    fn get_block_hash(&self, block_number: u64) -> Result<Option<H256>> {
        Ok(self.store.get(&key(BLOCK_HASH, &block_number))?)
    }

    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>> {
        Ok(self.store.get(&key(BLOCK, &block_hash))?)
    }

    fn get_transaction(&self, tx_hash: H256) -> Result<Option<SignedTransaction>> {
        Ok(self.store.get(&key(TRANSACTION, &tx_hash))?)
    }

    fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<TransactionReceipt>> {
        Ok(self.store.get(&key(TRANSACTION_RECEIPT, &tx_hash))?)
    }

    fn set_tip_block(&self, block_hash: H256) -> Result<()> {
        self.store.insert(TIP_BLOCK, block_hash)?;
        Ok(())
    }

    fn insert_block(&self, block: Block) -> Result<()> {
        let block_hash = block.block_hash();
        let block_number = block.header.number;
        self.store.insert(&key(BLOCK, &block_hash), block)?;
        self.store
            .insert(&key(BLOCK_HASH, &block_number), block_hash)?;
        Ok(())
    }

    fn insert_transaction(&self, tx: SignedTransaction) -> Result<()> {
        self.store.insert(&key(TRANSACTION, &tx.tx_hash()), tx)?;
        Ok(())
    }

    fn insert_transaction_receipt(
        &self,
        tx_hash: H256,
        tx_receipt: TransactionReceipt,
    ) -> Result<()> {
        self.store
            .insert(&key(TRANSACTION_RECEIPT, &tx_hash), tx_receipt)?;
        Ok(())
    }
}

pub fn key<K: Serialize + Hash>(prefix: &'static str, key: &K) -> H256 {
    let mut buf = [0u8; 32];
    let mut blake2b = Blake2bBuilder::new(32).personal(b"zk pika! pi~~~").build();

    blake2b.update(prefix.as_bytes());
    blake2b.update(&key.hash().0);
    blake2b.finalize(&mut buf);

    buf.into()
}
