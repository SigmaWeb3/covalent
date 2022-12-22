use std::time::SystemTime;

use anyhow::Result;
use primitive_types::H256;

use crate::types::{Block, BlockHeader, NumberHash};

use super::{
    chain::{Chain, ChannelChain},
    store::Store,
};

pub fn init(store: Store) -> Result<()> {
    let chain = ChannelChain::new(store);
    if chain.get_block(NumberHash::Number(0))?.is_some() {
        return Ok(());
    }

    let header = BlockHeader {
        number: 0,
        parent_hash: H256::zero(),
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        state_root: H256::zero(),
        transaction_root: H256::zero(),
        receipt_root: H256::zero(),
    };

    let block = Block {
        header,
        txs: vec![],
    };

    let block_hash = block.block_hash();
    chain.insert_block(block)?;
    chain.set_tip_block(block_hash)?;

    Ok(())
}
