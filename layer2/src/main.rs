#![allow(dead_code)]

mod api;
mod chain;
mod config;
mod consensus;
mod executor;
mod mempool;
mod merkle;
mod primitive;
mod trie;
mod types;

use std::sync::Arc;

use clap::{Arg, Command};

use crate::api::{run_jsonrpc_server, RpcImpl};
use crate::chain::CovalentChain;
use crate::config::{parse_file, Config};
use crate::consensus::Consensus;
use crate::mempool::MemPoolImpl;
use crate::trie::RocksTrieDB;

const MEMPOOL_SIZE: usize = 100;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    env_logger::init();
    let matches = Command::new("covalent-layer2")
        .arg(
            Arg::new("config_path")
                .long("config")
                .short('c')
                .default_value("./config/covalent.toml"),
        )
        .get_matches();

    let config: Config = parse_file(matches.get_one::<String>("config_path").unwrap()).unwrap();

    let chain = Arc::new(CovalentChain::new(config.chain_db_path()));
    let trie_db = Arc::new(RocksTrieDB::new(config.trie_db_path()));
    let mempool = Arc::new(MemPoolImpl::new(MEMPOOL_SIZE, config.chain_id()));
    let consensus = Consensus::new(
        Arc::clone(&trie_db),
        Arc::clone(&mempool),
        Arc::clone(&chain),
        config.chain_id(),
        config.address,
    );
    let rpc = RpcImpl::new(trie_db, chain, mempool);

    println!("jsonrpc server start");
    run_jsonrpc_server(rpc, config.rpc_uri).await;

    println!("covalent layer2 start");
    consensus.run().await;
}
