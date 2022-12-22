use std::collections::BTreeMap;

use anyhow::Result;
use primitive_types::{H160, H256, U256};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, Secp256k1,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use crate::{
    auxiliaries::{
        common::{cbmt_merkle_root, H256Ext},
        smt::{MemStore, SMT},
        store::Store,
    },
    types::{
        Channel, ChannelState, CloseChannel, CreateChannel, ExecutionExitCode, RawTransaction,
        Signature, TransactionReceipt, UpdateChannel,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("{0}")]
    SMT(sparse_merkle_tree::error::Error),
}

impl From<sparse_merkle_tree::error::Error> for ExecutionError {
    fn from(err: sparse_merkle_tree::error::Error) -> Self {
        ExecutionError::SMT(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub state_root: H256,
    pub receipt_root: H256,
    pub transaction_receipts: Vec<TransactionReceipt>,
    pub updated_channels: BTreeMap<H256, Channel>,
}

pub trait Executor {
    fn exec(&self, transactions: &Vec<RawTransaction>) -> Result<ExecutionReceipt, ExecutionError>;
}

pub struct ChannelExecutor {
    store: Store,
}

impl ChannelExecutor {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Executor for ChannelExecutor {
    fn exec(&self, transactions: &Vec<RawTransaction>) -> Result<ExecutionReceipt, ExecutionError> {
        let snap = MemStore::new(self.store.clone());
        let mut smt = SMT::new_with_store(snap)?;

        let mut receipts = Vec::with_capacity(transactions.len());
        for tx in transactions {
            let receipt = match tx {
                RawTransaction::CreateChannel(args) => create_channel(&mut smt, args)?,
                RawTransaction::UpdateChannel(args) => update_channel(&mut smt, args)?,
                RawTransaction::CloseChannel(args) => close_channel(&mut smt, args)?,
            };
            receipts.push(receipt);
        }

        let exec_receipt = ExecutionReceipt {
            state_root: smt.root().to_h256(),
            receipt_root: cbmt_merkle_root(&receipts),
            transaction_receipts: receipts,
            updated_channels: smt.take_store().take_leaves(),
        };

        Ok(exec_receipt)
    }
}

fn create_channel(
    smt: &mut SMT<MemStore>,
    args: &CreateChannel,
) -> Result<TransactionReceipt, ExecutionError> {
    if smt.get(&args.id.to_h256())?.exists() {
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorChannelExists);
        return Ok(receipt);
    }

    let total_balance =
        { args.balance2.iter() }.fold(U256::zero(), |accu, balance| accu + balance.settled);

    let channel = Channel {
        id: args.id,
        token: args.token.clone(),
        challenge_blocks: args.challenge_blocks,
        participant2: args.participant2,

        state: ChannelState::Open,
        version: 0u64,
        total_balance,
        balance2: args.balance2.clone(),
    };

    let root = smt.update(args.id.to_h256(), channel)?;
    let receipt = TransactionReceipt::success(H256Ext::to_h256(root));

    Ok(receipt)
}

fn update_channel(
    smt: &mut SMT<MemStore>,
    args: &UpdateChannel,
) -> Result<TransactionReceipt, ExecutionError> {
    let channel = smt.get(&args.channel_id.to_h256())?;
    if !channel.exists() {
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorChannelNotFound);
        return Ok(receipt);
    }
    if args.version <= channel.version {
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorRollbackChannelVersion);
        return Ok(receipt);
    }

    // Verify participant2 signatures
    let sig_msg = args.sig_msg();
    if let Err(_err) = verify_signature2(sig_msg, &channel.participant2, &args.signature2) {
        // eprintln!("verify signature2 err {}", err);
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorUpdateChannelSignature);
        return Ok(receipt);
    }

    let updated = Channel {
        version: args.version,
        balance2: args.balance2.clone(),
        ..channel
    };

    let root = smt.update(channel.id.to_h256(), updated)?;
    let receipt = TransactionReceipt::success(H256Ext::to_h256(root));

    Ok(receipt)
}

fn close_channel(
    smt: &mut SMT<MemStore>,
    args: &CloseChannel,
) -> Result<TransactionReceipt, ExecutionError> {
    let channel = smt.get(&args.channel_id.to_h256())?;
    if !channel.exists() {
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorChannelNotFound);
        return Ok(receipt);
    }
    if args.version <= channel.version {
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorRollbackChannelVersion);
        return Ok(receipt);
    }

    // Verify participant2 signatures
    let sig_msg = args.sig_msg();
    if let Err(_err) = verify_signature2(sig_msg, &channel.participant2, &args.signature2) {
        // eprintln!("verify signature2 err {}", err);
        let receipt = TransactionReceipt::err_res(ExecutionExitCode::ErrorUpdateChannelSignature);
        return Ok(receipt);
    }

    let closed = Channel {
        version: args.version,
        ..channel
    };

    let root = smt.update(channel.id.to_h256(), closed)?;
    let receipt = TransactionReceipt::success(H256Ext::to_h256(root));

    Ok(receipt)
}

#[derive(Debug, thiserror::Error)]
enum SignatureError {
    #[error("invalid signature length")]
    InvalidSignatureLength,
    #[error("{0}")]
    Secp256k1(#[from] secp256k1::Error),
    #[error("participant address not found")]
    ParticipantAddressNotFound,
}

fn extract_rec_id(rec_id: u8) -> Result<RecoveryId, SignatureError> {
    let param = match rec_id {
        r if r == 27 => 0,
        r if r == 28 => 1,
        r => r,
    };
    Ok(RecoveryId::from_i32(param.into())?)
}

fn verify_signature2(
    msg: H256,
    participant2: &[H160; 2],
    sig2: &[Signature; 2],
) -> Result<(), SignatureError> {
    let msg = Message::from_slice(&msg.0)?;
    let secp = Secp256k1::new();

    for sig in sig2 {
        let sig: [u8; 65] = sig
            .as_slice()
            .try_into()
            .map_err(|_| SignatureError::InvalidSignatureLength)?;

        let rec_id = extract_rec_id(sig[64])?;
        let rec_sig = RecoverableSignature::from_compact(&sig[..64], rec_id)?;

        let pk = secp.recover_ecdsa(&msg, &rec_sig)?;

        let mut hasher = Keccak256::new();
        hasher.update(&pk.serialize_uncompressed()[1..]);
        let rec_addr = &hasher.finalize()[12..];

        if !participant2.into_iter().any(|addr| addr.0 == rec_addr) {
            return Err(SignatureError::ParticipantAddressNotFound);
        }
    }

    Ok(())
}
