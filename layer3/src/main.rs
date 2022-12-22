use auxiliaries::{
    chain::ChannelChain, common::H256Ext, genesis, mempool::ChannelMempool, oracle::ChannelOracle,
    relay::ChannelRelay, smt::SMT, store::Store, wallet::Wallet,
};
use consensus::{ChannelConsensus, Consensus};
use primitive_types::U256;
use settlement::ChannelSettlement;
use tempfile::tempdir;
use types::{Balance, CreateChannel, RawTransaction, Token};

use crate::{
    auxiliaries::{mempool::MemPool, relayer::Relayer},
    settlement::Settlement,
    types::{ChannelState, CloseChannel, UpdateChannel},
};

mod auxiliaries;
mod consensus;
mod executor;
mod settlement;
mod types;

fn main() {
    let tmp_dir = tempdir().unwrap();
    let store = Store::open(tmp_dir).unwrap();
    genesis::init(store.clone()).unwrap();

    let mempool = ChannelMempool::default();
    let oracle = ChannelOracle::new(store.clone());
    let relay = ChannelRelay::new(mempool.clone(), oracle.clone());
    let chain = ChannelChain::new(store.clone());

    let consensus = ChannelConsensus::new(mempool.clone(), store.clone());
    let settlement = ChannelSettlement::new(store.clone(), oracle.clone(), relay.clone());

    let alice = Wallet::random();
    let bob = Wallet::random();
    let relayer = Relayer::new(oracle.clone(), relay.clone());
    println!("alice {:x}", alice.addr());
    println!("bob {:x}", bob.addr());
    println!("relayer {:x}", relayer.addr());

    // create test channel
    let channel_id: U256 = 2077u32.into();
    let create_channel = CreateChannel {
        id: channel_id,
        token: test_token(1u32.into()),
        participant2: [alice.addr(), bob.addr()],
        balance2: [Balance::new(100), Balance::new(0)],
    };
    oracle
        .set_pending_l2_create_channels(vec![create_channel.clone()])
        .unwrap();
    relayer.relay_l2_create_channel();

    let receipt = consensus.produce_block().unwrap();
    chain.apply_consensus_receipt(&receipt).unwrap();
    settlement.submit_block().unwrap();
    mempool.reset(&receipt.block).unwrap();

    let smt = SMT::new_with_store(store.clone()).unwrap();
    let channel = smt.get(&channel_id.to_h256()).unwrap();
    assert_eq!(channel.state, ChannelState::Open);
    assert_eq!(channel.balance2[0], Balance::new(100));

    // update test channel
    let mut update_channel = UpdateChannel {
        channel_id,
        version: 1,
        balance2: [Balance::new(50), Balance::new(50)],
        ..Default::default()
    };
    let sig_msg = update_channel.sig_msg();
    update_channel.signature2 = [
        alice.sign(sig_msg).unwrap().to_vec(),
        bob.sign(sig_msg).unwrap().to_vec(),
    ];

    let update_channel_tx = alice
        .sign_tx(RawTransaction::UpdateChannel(update_channel.clone()))
        .unwrap();
    mempool.push_transaction(update_channel_tx).unwrap();

    let receipt = consensus.produce_block().unwrap();
    chain.apply_consensus_receipt(&receipt).unwrap();
    settlement.submit_block().unwrap();
    mempool.reset(&receipt.block).unwrap();

    let smt = SMT::new_with_store(store.clone()).unwrap();
    let channel = smt.get(&channel_id.to_h256()).unwrap();
    assert_eq!(channel.balance2[0], Balance::new(50));
    assert_eq!(channel.balance2[1], Balance::new(50));

    // close test channel
    let mut close_channel = CloseChannel {
        channel_id,
        version: 2,
        ..Default::default()
    };
    let sig_msg = close_channel.sig_msg();
    close_channel.signature2 = [
        alice.sign(sig_msg).unwrap().to_vec(),
        bob.sign(sig_msg).unwrap().to_vec(),
    ];

    let close_channel_tx = bob
        .sign_tx(RawTransaction::CloseChannel(close_channel.clone()))
        .unwrap();
    mempool.push_transaction(close_channel_tx).unwrap();

    let receipt = consensus.produce_block().unwrap();
    chain.apply_consensus_receipt(&receipt).unwrap();
    settlement.submit_block().unwrap();
    mempool.reset(&receipt.block).unwrap();

    let smt = SMT::new_with_store(store.clone()).unwrap();
    let channel = smt.get(&channel_id.to_h256()).unwrap();
    assert_eq!(channel.state, ChannelState::Closed);
    assert_eq!(channel.balance2[0], Balance::new(50));
    assert_eq!(channel.balance2[1], Balance::new(50));

    oracle.set_pending_l3_withdrawals(vec![channel_id]).unwrap();
    relayer.relay_l3_withdrawal();
}

fn test_token(id: U256) -> Token {
    Token {
        id,
        symbol: format!("test token {}", id).try_into().unwrap(),
        decimal: 18u32.into(),
    }
}
