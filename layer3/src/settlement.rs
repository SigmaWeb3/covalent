use anyhow::Result;

use crate::{
    auxiliaries::{
        chain::{Chain, ChannelChain},
        oracle::{ChannelOracle, Oracle},
        relay::{ChannelRelay, Relay},
        store::Store,
    },
    types::NumberHash,
};

pub trait Settlement {
    fn submit_block(&self) -> Result<()>;
}

pub struct ChannelSettlement {
    store: Store,
    oracle: ChannelOracle,
    relay: ChannelRelay,
}

impl ChannelSettlement {
    pub fn new(store: Store, oracle: ChannelOracle, relay: ChannelRelay) -> Self {
        Self {
            store,
            oracle,
            relay,
        }
    }
}

impl Settlement for ChannelSettlement {
    fn submit_block(&self) -> Result<()> {
        let confirmed_blocks = self.oracle.confirmed_l3_blocks()?;
        let chain = ChannelChain::new(self.store.clone());
        let next_block = match chain.get_block(NumberHash::Number(confirmed_blocks + 1))? {
            Some(block) => block,
            None => return Ok(()),
        };

        self.relay.submit_l3_blocks(vec![next_block])?;
        Ok(())
    }
}
