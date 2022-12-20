use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::time::interval;

use crate::chain::Chain;
use crate::executor::{Execute, Executor};
use crate::mempool::MemPool;
use crate::merkle::Merkle;
use crate::types::{Block, Hash, Header, SignedTransaction, H160, U128, U64};

const BLOCK_INTERVAL: u64 = 3; // second
const CYCLE_LIMIT: U64 = U64([30_000_000]);

pub struct Consensus<DB, M, C> {
    trie_db:  Arc<DB>,
    mempool:  Arc<M>,
    chain:    Arc<C>,
    state:    State,
    chain_id: U64,
    address:  H160,
}

impl<DB, M, C> Consensus<DB, M, C>
where
    DB: cita_trie::DB,
    M: MemPool,
    C: Chain,
{
    pub fn new(
        trie_db: Arc<DB>,
        mempool: Arc<M>,
        chain: Arc<C>,
        chain_id: U64,
        address: H160,
    ) -> Self {
        let state = State {
            next_number: U64::one(),
            prev_hash:   Hash::default(),
            state_root:  Hash::default(),
        };

        Consensus {
            trie_db,
            mempool,
            chain,
            state,
            chain_id,
            address,
        }
    }

    pub async fn run(mut self) {
        let mut timer = interval(Duration::from_secs(BLOCK_INTERVAL));

        loop {
            timer.tick().await;
            let txs = self.mempool.package(CYCLE_LIMIT).await.unwrap();
            let block = self.build_block(txs);
            let mut executor = Executor::new(Arc::clone(&self.trie_db));
            let resp = executor.exec(block.header.state_root, &block.txs);

            self.chain.save_block(block.clone()).await.unwrap();
            println!("[consensus] Block {:?}", block.header.number);

            self.state.next_number = block.header.number + U64::one();
            self.state.prev_hash = block.header_hash();
            self.state.state_root = resp.state_root;
        }
    }

    fn build_block(&self, txs: Vec<SignedTransaction>) -> Block {
        let header = Header {
            chain_id:         self.chain_id,
            number:           self.state.next_number,
            prev_hash:        self.state.prev_hash,
            timestamp:        time_now(),
            transaction_root: Merkle::from_hashes(txs.iter().map(|tx| tx.tx_hash).collect())
                .get_root_hash()
                .unwrap_or_default(),
            state_root:       self.state.state_root,
            cycles_limit:     CYCLE_LIMIT,
            proposer:         self.address,
        };

        Block { header, txs }
    }
}

pub struct State {
    pub next_number: U64,
    pub prev_hash:   Hash,
    pub state_root:  Hash,
}

fn time_now() -> U128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .into()
}
