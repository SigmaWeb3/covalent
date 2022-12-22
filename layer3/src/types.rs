use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};

use crate::auxiliaries::common::Hash;

pub type Signature = Vec<u8>;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct Token {
    pub id: U256,
    pub symbol: Vec<u8>,
    pub decimal: U256,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct Balance {
    pub settled: u128,
    // pub pending_transfer: u128,
}

impl Balance {
    pub fn new(settled: u128) -> Self {
        Self { settled }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
pub enum ChannelState {
    #[default]
    NonExists,
    Open,
    Challenge,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct Channel {
    pub id: U256,
    pub token: Token,
    // pub challenge_blocks: u64,
    pub participant2: [H160; 2],

    pub state: ChannelState,
    pub version: u64,
    pub total_balance: U256,
    pub balance2: [Balance; 2],
    // pub transaction_root: H256,
}

impl Channel {
    pub fn exists(&self) -> bool {
        self.state != ChannelState::NonExists
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateChannel {
    pub id: U256,
    pub token: Token,
    // pub challenge_blocks: u64,
    pub participant2: [H160; 2],
    pub balance2: [Balance; 2],
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct UpdateChannel {
    pub channel_id: U256,
    pub version: u64,
    pub balance2: [Balance; 2],
    // pub transaction_root: H256,
    pub signature2: [Signature; 2],
}

impl UpdateChannel {
    pub fn sig_msg(&self) -> H256 {
        let args = UpdateChannel {
            channel_id: self.channel_id,
            version: self.version,
            balance2: self.balance2.clone(),
            ..Default::default()
        };

        args.hash()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CloseChannel {
    pub channel_id: U256,
    pub version: u64,
    pub signature2: [Signature; 2],
}

impl CloseChannel {
    pub fn sig_msg(&self) -> H256 {
        let args = CloseChannel {
            channel_id: self.channel_id,
            version: self.version,
            ..Default::default()
        };

        args.hash()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transfer {
    pub channel_id: U256,
    pub to: H160,
    pub amount: u128,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RawTransaction {
    CreateChannel(CreateChannel),
    UpdateChannel(UpdateChannel),
    CloseChannel(CloseChannel),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignedTransaction {
    pub raw: RawTransaction,
    pub sig: Signature,

    // Cache only
    pub from: H160,
    pub hash: H256,
}

impl SignedTransaction {
    pub fn tx_hash(&self) -> H256 {
        self.raw.hash()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[repr(u8)]
pub enum ExecutionExitCode {
    Success = 0,
    ErrorChannelExists = 1,
    ErrorChannelNotFound = 2,
    ErrorRollbackChannelVersion = 3,
    ErrorUpdateChannelSignature = 4,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionReceipt {
    pub exit_code: ExecutionExitCode,
    pub state_root: H256,
}

impl TransactionReceipt {
    pub fn success(state_root: H256) -> Self {
        TransactionReceipt {
            exit_code: ExecutionExitCode::Success,
            state_root,
        }
    }

    pub fn err_res(exit_code: ExecutionExitCode) -> Self {
        TransactionReceipt {
            exit_code,
            state_root: H256::zero(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockHeader {
    pub number: u64,
    pub parent_hash: H256,
    pub timestamp: u128,
    pub state_root: H256,
    pub transaction_root: H256,
    pub receipt_root: H256,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<SignedTransaction>,
}

impl Block {
    pub fn block_hash(&self) -> H256 {
        self.header.hash()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NumberHash {
    Number(u64),
    Hash(H256),
}
