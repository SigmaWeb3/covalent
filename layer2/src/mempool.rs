use std::sync::RwLock;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use ophelia::{HashValue, SignatureVerify};
use ophelia_secp256k1::{Secp256k1PublicKey, Secp256k1Signature};
use rlp::Encodable;

use crate::types::{Hash, Hasher, SignedTransaction, TokenAction, U64};

const TX_CYCLE_LIMIT: U64 = U64([100_000]);

#[async_trait]
pub trait MemPool: Sync + Send {
    async fn insert(&self, stx: SignedTransaction) -> Result<()>;

    async fn package(&self, cycle_limit: U64) -> Result<Vec<SignedTransaction>>;

    async fn remove(&self, hashes: Vec<Hash>) -> Result<()>;
}

pub struct MemPoolImpl {
    tx_map:     DashMap<Hash, SignedTransaction>,
    flush_lock: RwLock<()>,
    chain_id:   U64,
}

#[async_trait]
impl MemPool for MemPoolImpl {
    async fn insert(&self, stx: SignedTransaction) -> Result<()> {
        self.verify_tx(&stx)?;
        let _insert = self.flush_lock.read();
        self.tx_map.insert(stx.tx_hash, stx);
        Ok(())
    }

    async fn package(&self, total_limit: U64) -> Result<Vec<SignedTransaction>> {
        let _package = self.flush_lock.write();
        let mut sum_cycle = U64::zero();

        Ok(self
            .tx_map
            .iter()
            .take_while(|kv| {
                let tx_limit = kv.value().cycle_limit();
                if total_limit >= (sum_cycle + tx_limit) {
                    sum_cycle += tx_limit;
                    true
                } else {
                    false
                }
            })
            .map(|kv| kv.value().clone())
            .collect::<Vec<_>>())
    }

    async fn remove(&self, hashes: Vec<Hash>) -> Result<()> {
        let _flush = self.flush_lock.write();
        hashes.iter().for_each(|hash| {
            let _ = self.tx_map.remove(hash);
        });
        Ok(())
    }
}

impl MemPoolImpl {
    pub fn new(pool_size: usize, id: U64) -> Self {
        MemPoolImpl {
            tx_map:     DashMap::with_capacity(pool_size),
            flush_lock: RwLock::new(()),
            chain_id:   id,
        }
    }

    fn verify_tx(&self, stx: &SignedTransaction) -> Result<()> {
        if stx.chain_id() != self.chain_id {
            return Err(anyhow!("Invalid chain id"));
        }

        if stx.cycle_limit() > TX_CYCLE_LIMIT {
            return Err(anyhow!("Exceed tx cycle limit"));
        }

        if Hasher::digest_(stx.raw.rlp_bytes()) != stx.tx_hash {
            return Err(anyhow!("Tx hash diff"));
        }

        if stx
            .raw
            .requests
            .iter()
            .any(|req| req.action == TokenAction::Transfer && req.to.is_none())
        {
            return Err(anyhow!("Invalid transfer request"));
        }

        Secp256k1Signature::try_from(stx.signature.to_vec().as_ref())
            .map_err(|_| anyhow!("Invalid signature"))?
            .verify(
                &HashValue::from_bytes_unchecked(stx.tx_hash.0),
                &Secp256k1PublicKey::try_from(stx.pub_key.to_vec().as_ref())
                    .map_err(|_| anyhow!("Invalid public key"))?,
            )
            .map_err(|_| anyhow!("Verify signature failed"))
    }
}
