pub use crate::primitive::{Hash, Hasher};
pub use bytes::Bytes;
pub use ethereum_types::{H160, U128, U256, U64};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use rlp::{Decodable, DecoderError, Encodable, Rlp};
use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, IntoPrimitive, TryFromPrimitive, Clone, Copy, Debug, PartialEq, Eq,
)]
#[repr(u8)]
pub enum TokenAction {
    Mint,
    Lock,
    Unlock,
    Divert,
}

impl Encodable for TokenAction {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(1).append(&(*self as u8));
    }
}

impl Decodable for TokenAction {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let act: u8 = rlp.val_at(0)?;
        TokenAction::try_from(act).map_err(|_| DecoderError::RlpExpectedToBeData)
    }
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub address:      H160,
    pub balance_root: Hash,
}

#[derive(
    Serialize, Deserialize, RlpEncodable, RlpDecodable, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct TokenBalance {
    pub locked: U256,
    pub active: U256,
}

impl TokenBalance {
    pub fn is_uninitialized(&self) -> bool {
        self.locked.is_zero() && self.active.is_zero()
    }
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct RawTransaction {
    pub chain_id:     U64,
    pub cycles_price: U64,
    pub cycles_limit: U64,
    pub nonce:        Hash,
    pub requests:     Vec<TransactionRequest>,
    pub timeout:      U64,
    pub sender:       H160,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct TransactionRequest {
    pub address:  H160,
    pub token_id: Hash,
    pub amount:   U256,
    pub action:   TokenAction,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct SignedTransaction {
    pub raw:       RawTransaction,
    pub tx_hash:   Hash,
    pub pub_key:   Bytes,
    pub signature: Bytes,
}

impl SignedTransaction {
    pub fn cycle_limit(&self) -> U64 {
        self.raw.cycles_limit
    }

    pub fn chain_id(&self) -> U64 {
        self.raw.chain_id
    }
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]

pub struct Header {
    pub chain_id:         U64,
    pub number:           U64,
    pub prev_hash:        Hash,
    pub timestamp:        U128,
    pub transaction_root: Hash,
    pub state_root:       Hash,
    pub cycles_limit:     U64,
    pub proposer:         H160,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub header: Header,
    pub txs:    Vec<SignedTransaction>,
}

impl Block {
    pub fn header_hash(&self) -> Hash {
        Hasher::digest_(self.header.rlp_bytes())
    }
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct BlockExecuteResponse {
    pub state_root: Hash,
    pub inner:      Vec<ExecuteResponse>,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]

pub struct ExecuteResponse {
    pub tx_hash: Hash,
    pub ret:     Vec<u8>,
    pub error:   Option<ExecuteError>,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct ExecuteError {
    pub error_code:    u32,
    pub error_message: String,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct TransactionReceipt {
    pub tx_hash:    Hash,
    pub state_root: Hash,
    pub logs:       Vec<Log>,
}

#[derive(Serialize, Deserialize, RlpEncodable, RlpDecodable, Clone, Debug, PartialEq, Eq)]
pub struct Log {
    name: String,
    data: String,
}

impl Log {
    pub fn new(name: String, data: String) -> Self {
        Log { name, data }
    }
}
