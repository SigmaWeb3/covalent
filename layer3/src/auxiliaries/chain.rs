use anyhow::Result;
use primitive_types::{H256, U256};

use crate::types::{Block, Channel, NumberHash, SignedTransaction};

pub trait Chain {
    fn tip_block(&self) -> Result<NumberHash>;
    fn save_block(&self, block: Block) -> Result<()>;
    fn get_block(&self, number_hash: NumberHash) -> Result<Block>;
    fn get_channel(&self, channel_id: U256) -> Result<Channel>;
    fn get_transaction(&self, tx_hash: H256) -> Result<SignedTransaction>;
}
