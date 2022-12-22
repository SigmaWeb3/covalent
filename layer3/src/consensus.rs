use std::collections::BTreeMap;

use anyhow::Result;
use primitive_types::H256;

use crate::{
    auxiliaries::{
        mempool::{ChannelMap, MemPool},
        store::Store,
    },
    executor::{ChannelExecutor, ExecutionReceipt, Executor},
    types::{Block, Channel, TransactionReceipt},
};

pub struct ConsensusReceipt {
    pub block: Block,

    // Cache
    pub transaction_receipts: Vec<TransactionReceipt>,
    pub updated_channels: BTreeMap<H256, Channel>,
}

pub trait Consensus {
    fn produce_block(&self) -> Result<ConsensusReceipt>;
}

pub struct ChannelConsensus {
    mempool: ChannelMap,
    store: Store,
}

impl Consensus for ChannelConsensus {
    fn produce_block(&self) -> Result<ConsensusReceipt> {
        let executor = ChannelExecutor::new(self.store.clone());

        let txs = self.mempool.package_transactions()?;
        let exec_receipt = executor.exec(&txs.iter().map(|s| s.raw).collect())?;

        todo!()
    }
}
