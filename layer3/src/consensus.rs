use std::{collections::BTreeMap, time::SystemTime};

use anyhow::Result;
use primitive_types::H256;

use crate::{
    auxiliaries::{
        chain::{Chain, ChannelChain},
        common::{cbmt_merkle_root, Hash},
        mempool::{ChannelMempool, MemPool},
        store::Store,
    },
    executor::{ChannelExecutor, Executor},
    types::{Block, BlockHeader, Channel, TransactionReceipt},
};

#[derive(Debug)]
pub struct ConsensusReceipt {
    pub block: Block,

    // Cache
    pub transaction_receipts: Vec<TransactionReceipt>,
    pub updated_channels: BTreeMap<H256, Channel>,
}

impl ConsensusReceipt {
    pub fn tx_receipts(&self) -> Vec<(H256, TransactionReceipt)> {
        { self.block.txs.iter() }
            .zip(self.transaction_receipts.iter())
            .map(|(tx, receipt)| (tx.tx_hash(), receipt.clone()))
            .collect()
    }
}

pub trait Consensus {
    fn produce_block(&self) -> Result<ConsensusReceipt>;
}

pub struct ChannelConsensus {
    mempool: ChannelMempool,
    store: Store,
}

impl ChannelConsensus {
    pub fn new(mempool: ChannelMempool, store: Store) -> Self {
        Self { mempool, store }
    }
}

impl Consensus for ChannelConsensus {
    fn produce_block(&self) -> Result<ConsensusReceipt> {
        let executor = ChannelExecutor::new(self.store.clone());

        let txs = self.mempool.package_transactions()?;
        let exec_receipt = executor.exec(&txs.iter().map(|s| s.raw.clone()).collect())?;
        let transaction_root = cbmt_merkle_root(&txs.iter().map(|t| t.raw.hash()).collect());

        let chain = ChannelChain::new(self.store.clone());
        let tip_block = chain.tip_block()?;

        let next_block = {
            let header = BlockHeader {
                number: tip_block.header.number + 1,
                parent_hash: tip_block.block_hash(),
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
                state_root: exec_receipt.state_root,
                transaction_root,
                receipt_root: exec_receipt.receipt_root,
            };

            Block { header, txs }
        };

        let receipt = ConsensusReceipt {
            block: next_block,
            transaction_receipts: exec_receipt.transaction_receipts,
            updated_channels: exec_receipt.updated_channels,
        };

        Ok(receipt)
    }
}
