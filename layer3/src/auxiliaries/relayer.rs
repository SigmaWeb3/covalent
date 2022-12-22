use primitive_types::H160;

use crate::types::RawTransaction;

use super::{
    oracle::{ChannelOracle, Oracle},
    relay::{ChannelRelay, Relay},
    wallet::Wallet,
};

#[derive(Clone)]
pub struct Relayer {
    wallet: Wallet,
    oracle: ChannelOracle,
    relay: ChannelRelay,
}

impl Relayer {
    pub fn new(oracle: ChannelOracle, relay: ChannelRelay) -> Self {
        Self {
            wallet: Wallet::random(),
            oracle,
            relay,
        }
    }

    pub fn addr(&self) -> H160 {
        self.wallet.addr()
    }

    pub fn relay_l2_create_channel(&self) {
        let create_channels = self.oracle.pending_l2_create_channels().unwrap();
        for create_channel in create_channels {
            let tx = { &self.wallet }
                .sign_tx(RawTransaction::CreateChannel(create_channel))
                .unwrap();
            self.relay.submit_l2_create_channel(tx).unwrap();
        }
        self.oracle.set_pending_l2_create_channels(vec![]).unwrap();
    }

    pub fn relay_l3_withdrawal(&self) {
        let _closed_channel_ids = self.oracle.pending_l3_withdrawals().unwrap();
        // create l2 unlock and transfer tx
        //self.relay.relay_l3_withdrawal(tx).unwrap();
    }
}
