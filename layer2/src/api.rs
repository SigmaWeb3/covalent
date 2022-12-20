use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use jsonrpsee::core::{Error, RpcResult};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;

use crate::chain::Chain;
use crate::executor::Executor;
use crate::mempool::MemPool;
use crate::types::{Block, Hash, SignedTransaction, TokenBalance, H160, U64};

#[rpc(server)]
pub trait Rpc {
    #[method(name = "send_transaction")]
    async fn send_transaction(&self, stx: SignedTransaction) -> RpcResult<()>;

    #[method(name = "get_block_by_number")]
    async fn get_block_by_number(&self, number: U64) -> RpcResult<Option<Block>>;

    #[method(name = "get_transaction_by_hash")]
    async fn get_transaction_by_hash(&self, hash: Hash) -> RpcResult<Option<SignedTransaction>>;

    #[method(name = "get_balance")]
    async fn get_balance(&self, address: H160, token_id: Hash) -> RpcResult<TokenBalance>;
}

pub struct RpcImpl<DB, C, M> {
    trie_db: Arc<DB>,
    chain:   Arc<C>,
    mempool: Arc<M>,
}

#[async_trait]
impl<DB, C, M> RpcServer for RpcImpl<DB, C, M>
where
    DB: cita_trie::DB + Sync + 'static,
    C: Chain + 'static,
    M: MemPool + 'static,
{
    async fn send_transaction(&self, stx: SignedTransaction) -> RpcResult<()> {
        self.mempool
            .insert(stx)
            .await
            .map_err(|e| Error::Custom(e.to_string()))
    }

    async fn get_block_by_number(&self, number: U64) -> RpcResult<Option<Block>> {
        self.chain
            .get_block_by_number(&number)
            .await
            .map_err(|e| Error::Custom(e.to_string()))
    }

    async fn get_transaction_by_hash(&self, hash: Hash) -> RpcResult<Option<SignedTransaction>> {
        self.chain
            .get_tx_by_hash(&hash)
            .await
            .map_err(|e| Error::Custom(e.to_string()))
    }

    async fn get_balance(&self, address: H160, token_id: Hash) -> RpcResult<TokenBalance> {
        let header = self
            .chain
            .get_latest_block()
            .await
            .map_err(|e| Error::Custom(e.to_string()))?;
        let executor = Executor::new(Arc::clone(&self.trie_db));

        Ok(executor.get_balance(
            &executor.trie(
                &executor
                    .get_account(&executor.trie(&header.state_root), &address)
                    .balance_root,
            ),
            &token_id,
        ))
    }
}

impl<DB, C, M> RpcImpl<DB, C, M>
where
    DB: cita_trie::DB + Sync + 'static,
    C: Chain + 'static,
    M: MemPool + 'static,
{
    pub fn new(trie_db: Arc<DB>, chain: Arc<C>, mempool: Arc<M>) -> Self {
        RpcImpl {
            trie_db,
            chain,
            mempool,
        }
    }
}

pub async fn run_jsonrpc_server<RPC: RpcServer>(rpc_impl: RPC, uri: SocketAddr) {
    let server = ServerBuilder::default().build(uri).await.unwrap();
    let _handle = server.start(rpc_impl.into_rpc()).unwrap();
}
