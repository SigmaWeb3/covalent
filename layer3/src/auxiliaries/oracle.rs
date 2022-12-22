use anyhow::Result;
use primitive_types::U256;

use crate::types::CreateChannel;

use super::store::Store;

type ChannelId = U256;

pub trait Oracle {
    fn confirmed_l3_blocks(&self) -> Result<u64>;
    fn confirmed_l3_withdrawals(&self) -> Result<Vec<ChannelId>>;
    fn pending_l2_create_channels(&self) -> Result<Vec<CreateChannel>>;
    fn pending_l3_withdrawals(&self) -> Result<Vec<ChannelId>>;
}

#[derive(Clone)]
pub struct ChannelOracle {
    store: Store,
}

impl ChannelOracle {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /** test **/
    const CONFIRMED_L3_BLOCKS: &str = "CONFIRMED_L3_BLOCKS";
    pub fn set_confirmed_l3_blocks(&self, blocks: u64) -> Result<()> {
        self.store.insert(&Self::CONFIRMED_L3_BLOCKS, blocks)?;
        Ok(())
    }

    const CONFIRMED_L3_WITHDRAWALS: &str = "CONFIRMED_L3_WITHDRAWALS";
    pub fn set_confirmed_l3_withdrawals(&self, channels: Vec<ChannelId>) -> Result<()> {
        self.store
            .insert(&Self::CONFIRMED_L3_WITHDRAWALS, channels)?;
        Ok(())
    }

    const PENDING_L3_WITHDRAWALS: &str = "PENDING_L3_WITHDRAWALS";
    pub fn set_pending_l3_withdrawals(&self, channels: Vec<ChannelId>) -> Result<()> {
        self.store
            .insert(&Self::CONFIRMED_L3_WITHDRAWALS, channels)?;
        Ok(())
    }

    const PENDING_CREATE_CHANNELS: &str = "PENDING_CREATE_CHANNELS";
    pub fn set_pending_l2_create_channels(
        &self,
        create_channels: Vec<CreateChannel>,
    ) -> Result<()> {
        self.store
            .insert(&Self::PENDING_CREATE_CHANNELS, create_channels)?;
        Ok(())
    }
}

impl Oracle for ChannelOracle {
    fn confirmed_l3_blocks(&self) -> Result<u64> {
        Ok(self.store.get(&Self::CONFIRMED_L3_BLOCKS)?.unwrap_or(0))
    }

    fn confirmed_l3_withdrawals(&self) -> Result<Vec<ChannelId>> {
        Ok({ &self.store }
            .get(&Self::CONFIRMED_L3_WITHDRAWALS)?
            .unwrap_or_default())
    }

    fn pending_l2_create_channels(&self) -> Result<Vec<CreateChannel>> {
        Ok({ &self.store }
            .get(&Self::PENDING_CREATE_CHANNELS)?
            .unwrap_or_default())
    }

    fn pending_l3_withdrawals(&self) -> Result<Vec<ChannelId>> {
        Ok({ &self.store }
            .get(&Self::PENDING_L3_WITHDRAWALS)?
            .unwrap_or_default())
    }
}
