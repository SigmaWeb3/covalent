use crate::types::{Block, SignedTransaction};
use anyhow::Result;

use super::{
    mempool::{ChannelMempool, MemPool},
    oracle::ChannelOracle,
};

pub trait Relay {
    fn submit_l2_create_channel(&self, tx: SignedTransaction) -> Result<()>;
    fn submit_l3_blocks(&self, blocks: Vec<Block>) -> Result<()>;
    fn submit_l3_withdrawals(&self, tx: SignedTransaction) -> Result<()>;
}

#[derive(Clone)]
pub struct ChannelRelay {
    mempool: ChannelMempool,
    oracle: ChannelOracle,
}

impl ChannelRelay {
    pub fn new(mempool: ChannelMempool, oracle: ChannelOracle) -> Self {
        Self { mempool, oracle }
    }
}

impl Relay for ChannelRelay {
    fn submit_l2_create_channel(&self, tx: SignedTransaction) -> Result<()> {
        self.mempool.push_transaction(tx)
    }

    fn submit_l3_blocks(&self, blocks: Vec<Block>) -> Result<()> {
        if let Some(last_block) = blocks.last() {
            self.oracle
                .set_confirmed_l3_blocks(last_block.header.number)?;
        }

        Ok(())
    }

    fn submit_l3_withdrawals(&self, tx: SignedTransaction) -> Result<()> {
        match tx.raw {
            crate::types::RawTransaction::CloseChannel(args) => {
                self.oracle
                    .set_confirmed_l3_withdrawals(vec![args.channel_id])?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }
}
