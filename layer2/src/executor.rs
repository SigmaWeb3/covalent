use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use cita_trie::{PatriciaTrie, Trie};
use derive_more::Display;
use num_enum::IntoPrimitive;
use rlp::{Decodable, Rlp};

use crate::types::{
    Account, BlockExecuteResponse, ExecuteError, ExecuteResponse, Hash, Hasher, Log,
    SignedTransaction, TokenAction, TokenBalance, H160, U256,
};

type TxResult<T> = std::result::Result<T, ExecuteError>;

pub trait Execute {
    fn exec(&mut self, state_root: Hash, txs: Vec<SignedTransaction>) -> BlockExecuteResponse;
}

pub struct Executor<DB> {
    trie_db:          Arc<DB>,
    block_exec_cache: HashMap<H160, BTreeMap<Hash, TokenBalance>>,
    tx_exec_cache:    HashMap<H160, BTreeMap<Hash, TokenBalance>>,
    log_cache:        BTreeMap<Hash, Vec<Log>>,
}

impl<DB: cita_trie::DB> Execute for Executor<DB> {
    fn exec(&mut self, state_root: Hash, txs: Vec<SignedTransaction>) -> BlockExecuteResponse {
        let mut inner = Vec::with_capacity(txs.len());
        let state_trie = self.trie(&state_root);

        txs.iter().for_each(|stx| {
            let (res, err) = match self.inner_exec(stx, &state_trie) {
                Ok(resp) => (resp, None),
                Err(e) => (Vec::new(), Some(e)),
            };

            inner.push(ExecuteResponse {
                tx_hash: stx.tx_hash,
                ret:     res,
                error:   err,
            });
        });

        BlockExecuteResponse { inner }
    }
}

impl<DB: cita_trie::DB> Executor<DB> {
    pub fn new(db: Arc<DB>) -> Self {
        Executor {
            trie_db:          db,
            log_cache:        BTreeMap::new(),
            block_exec_cache: HashMap::new(),
            tx_exec_cache:    HashMap::new(),
        }
    }

    fn inner_exec(
        &mut self,
        stx: &SignedTransaction,
        state_trie: &PatriciaTrie<DB, Hasher>,
    ) -> TxResult<Vec<u8>> {
        for req in stx.raw.requests.iter() {
            let account = self.get_account(state_trie, &req.address);
            let balance_trie = self.trie(&account.balance_root);
            let record = self.get_balance(&balance_trie, &req.token_id);
            let rec = self
                .tx_exec_cache
                .entry(req.address)
                .or_insert_with(BTreeMap::new)
                .entry(req.token_id)
                .or_default();
            if rec.is_uninitialized() {
                *rec = record;
            }
            let log_map = self.log_cache.entry(stx.tx_hash).or_insert_with(Vec::new);
            let addr_str = req.address.to_string();

            match req.action {
                TokenAction::Mint => {
                    rec.active += req.amount;

                    log_map.push(Log::new(
                        addr_str,
                        gen_log(FlowDirection::ActiveAdd, req.amount),
                    ));
                }
                TokenAction::Lock => {
                    if rec.active < req.amount {
                        return Err(TransactionError::ActiveAmountLessThanLockReq.into());
                    }

                    rec.active -= req.amount;
                    rec.locked += req.amount;

                    log_map.push(Log::new(
                        addr_str,
                        gen_log(FlowDirection::ActiveToLock, req.amount),
                    ));
                }
                TokenAction::Unlock => {
                    if rec.locked < req.amount {
                        return Err(TransactionError::LockedAmountLessThanUnlockReq.into());
                    }

                    rec.locked -= req.amount;
                    rec.active += req.amount;

                    log_map.push(Log::new(
                        addr_str,
                        gen_log(FlowDirection::LockToActive, req.amount),
                    ));
                }
                TokenAction::Divert => {
                    if rec.active < req.amount {
                        return Err(TransactionError::ActiveAmountLessThanDivertReq.into());
                    }

                    rec.active -= req.amount;

                    log_map.push(Log::new(
                        addr_str,
                        gen_log(FlowDirection::ActiveReduce, req.amount),
                    ));
                }
            }
        }

        Ok(rlp::encode(&gen_resp(stx.tx_hash)).to_vec())
    }

    fn trie(&self, root: &Hash) -> PatriciaTrie<DB, Hasher> {
        let hasher = Arc::new(Hasher::default());
        if root.is_zero() {
            return PatriciaTrie::new(Arc::clone(&self.trie_db), hasher);
        }

        PatriciaTrie::from(Arc::clone(&self.trie_db), hasher, root.as_bytes())
            .expect("trie from root")
    }

    fn get_account(&self, trie: &PatriciaTrie<DB, Hasher>, addr: &H160) -> Account {
        if let Some(raw) = trie.get(addr.as_bytes()).expect("get account") {
            if let Ok(account) = Account::decode(&Rlp::new(&raw)) {
                return account;
            }
        }

        Account {
            address:      *addr,
            balance_root: Hash::default(),
        }
    }

    fn get_balance(&self, trie: &PatriciaTrie<DB, Hasher>, token_id: &Hash) -> TokenBalance {
        if let Some(raw) = trie.get(token_id.as_bytes()).expect("get account") {
            if let Ok(balance) = TokenBalance::decode(&Rlp::new(&raw)) {
                return balance;
            }
        }

        TokenBalance::default()
    }

    fn clear_tx_cache(&mut self) {
        self.tx_exec_cache.clear();
    }
}

fn gen_log(flow: FlowDirection, amount: U256) -> String {
    format!("{} {}", flow, amount)
}

fn gen_resp(tx_hash: Hash) -> String {
    format!("tx {} success", tx_hash)
}

#[derive(Display)]
enum FlowDirection {
    #[display(fmt = "active add")]
    ActiveAdd,
    #[display(fmt = "active to lock")]
    ActiveToLock,
    #[display(fmt = "lock to active")]
    LockToActive,
    #[display(fmt = "active reduce")]
    ActiveReduce,
}

#[derive(Display, IntoPrimitive, Clone, Copy, Debug)]
#[repr(u32)]
enum TransactionError {
    ActiveAmountLessThanLockReq,
    LockedAmountLessThanUnlockReq,
    ActiveAmountLessThanDivertReq,
}

impl From<TransactionError> for ExecuteError {
    fn from(e: TransactionError) -> Self {
        ExecuteError {
            error_code:    e.into(),
            error_message: e.to_string(),
        }
    }
}
