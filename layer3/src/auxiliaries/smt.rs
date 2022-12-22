use std::collections::BTreeMap;

use primitive_types::H256;
use serde::{Deserialize, Serialize};
use sparse_merkle_tree::{
    blake2b::Blake2bHasher,
    error::Error as SMTError,
    merge::MergeValue,
    traits::{StoreReadOps, StoreWriteOps, Value},
    BranchKey, BranchNode, SparseMerkleTree, H256 as SMTH256,
};

use crate::{
    auxiliaries::{
        common::{H256Ext, Hash},
        store::{Store, StoreError},
    },
    types::Channel,
};

pub type SMT<S> = SparseMerkleTree<Blake2bHasher, Channel, S>;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub SMTError);

impl Value for Channel {
    fn to_h256(&self) -> SMTH256 {
        self.hash().0.into()
    }

    fn zero() -> Self {
        Default::default()
    }
}

pub struct MemStore {
    store: Store,
    overlay: Overlay,
}

impl MemStore {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            overlay: Default::default(),
        }
    }

    pub fn take_leaves(self) -> BTreeMap<H256, Channel> {
        self.overlay
            .leaves
            .into_iter()
            .map(|(k, v)| (H256Ext::to_h256(&k), v))
            .collect()
    }
}

impl StoreReadOps<Channel> for MemStore {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, SMTError> {
        match self.overlay.branches.get(branch_key) {
            Some(v) => Ok(Some(v.clone().into())),
            None => self.store.get_branch(branch_key),
        }
    }

    fn get_leaf(&self, leaf_key: &SMTH256) -> Result<Option<Channel>, SMTError> {
        match self.overlay.leaves.get(leaf_key) {
            Some(v) => Ok(Some(v.clone().into())),
            None => self.store.get_leaf(leaf_key),
        }
    }
}

impl StoreWriteOps<Channel> for MemStore {
    fn insert_branch(&mut self, node_key: BranchKey, branch: BranchNode) -> Result<(), SMTError> {
        self.overlay.branches.insert(node_key, branch);
        Ok(())
    }

    fn insert_leaf(&mut self, leaf_key: SMTH256, leaf: Channel) -> Result<(), SMTError> {
        self.overlay.leaves.insert(leaf_key, leaf);
        Ok(())
    }

    fn remove_branch(&mut self, node_key: &BranchKey) -> Result<(), SMTError> {
        self.overlay.branches.remove(node_key);
        Ok(())
    }

    fn remove_leaf(&mut self, leaf_key: &SMTH256) -> Result<(), SMTError> {
        self.overlay.leaves.remove(leaf_key);
        Ok(())
    }
}

#[derive(Default)]
struct Overlay {
    branches: BTreeMap<BranchKey, BranchNode>,
    leaves: BTreeMap<SMTH256, Channel>,
}

impl From<StoreError> for SMTError {
    fn from(err: StoreError) -> Self {
        SMTError::Store(err.to_string())
    }
}

impl StoreReadOps<Channel> for Store {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, SMTError> {
        self.get::<_, SMTBranchNode>(&SMTBranchKey::from(branch_key))?
            .map(|opt| Ok(opt.into()))
            .transpose()
    }

    fn get_leaf(&self, leaf_key: &SMTH256) -> Result<Option<Channel>, SMTError> {
        Ok(self.get::<H256, Channel>(&H256Ext::to_h256(leaf_key))?)
    }
}

impl StoreWriteOps<Channel> for Store {
    fn insert_branch(&mut self, node_key: BranchKey, branch: BranchNode) -> Result<(), SMTError> {
        self.insert(SMTBranchKey::from(&node_key), SMTBranchNode::from(branch))?;
        Ok(())
    }

    fn insert_leaf(&mut self, leaf_key: SMTH256, leaf: Channel) -> Result<(), SMTError> {
        self.insert::<H256, _>(H256Ext::to_h256(&leaf_key), leaf)?;
        Ok(())
    }

    fn remove_branch(&mut self, node_key: &BranchKey) -> Result<(), SMTError> {
        self.remove(SMTBranchKey::from(node_key))?;
        Ok(())
    }

    fn remove_leaf(&mut self, leaf_key: &SMTH256) -> Result<(), SMTError> {
        self.remove::<H256>(H256Ext::to_h256(leaf_key))?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SMTBranchKey {
    height: u8,
    node_key: H256,
}

impl From<&BranchKey> for SMTBranchKey {
    fn from(key: &BranchKey) -> Self {
        Self {
            height: key.height,
            node_key: H256Ext::to_h256(&key.node_key),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
enum SMTMergeValue {
    Value(H256),
    MergeWithZero {
        base_node: H256,
        zero_bits: H256,
        zero_count: u8,
    },
    ShortCut {
        key: H256,
        value: H256,
        height: u8,
    },
}

impl From<MergeValue> for SMTMergeValue {
    fn from(value: MergeValue) -> Self {
        match value {
            MergeValue::Value(v) => SMTMergeValue::Value(<[u8; 32]>::from(v).into()),
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count,
            } => SMTMergeValue::MergeWithZero {
                base_node: H256Ext::to_h256(&base_node),
                zero_bits: H256Ext::to_h256(&zero_bits),
                zero_count,
            },
            MergeValue::ShortCut { key, value, height } => SMTMergeValue::ShortCut {
                key: H256Ext::to_h256(&key),
                value: H256Ext::to_h256(&value),
                height,
            },
        }
    }
}

impl From<SMTMergeValue> for MergeValue {
    fn from(value: SMTMergeValue) -> Self {
        match value {
            SMTMergeValue::Value(v) => MergeValue::Value(<[u8; 32]>::from(v).into()),
            SMTMergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count,
            } => MergeValue::MergeWithZero {
                base_node: base_node.to_h256(),
                zero_bits: zero_bits.to_h256(),
                zero_count,
            },
            SMTMergeValue::ShortCut { key, value, height } => MergeValue::ShortCut {
                key: key.to_h256(),
                value: value.to_h256(),
                height,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SMTBranchNode {
    left: SMTMergeValue,
    right: SMTMergeValue,
}

impl From<BranchNode> for SMTBranchNode {
    fn from(BranchNode { left, right }: BranchNode) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
        }
    }
}

impl From<SMTBranchNode> for BranchNode {
    fn from(SMTBranchNode { left, right }: SMTBranchNode) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use primitive_types::U256;
    use tempfile::tempdir;

    use crate::auxiliaries::common::H256Ext;

    use super::*;

    #[test]
    fn test_smt() {
        let tmp_db_path = tempdir().unwrap();
        let store = Store::open(tmp_db_path).unwrap();
        let mut smt = SMT::new_with_store(store).unwrap();
        assert_eq!(smt.root().as_slice(), H256::zero().0);

        let channel_id = U256::one();
        let channel = Channel {
            id: channel_id,
            ..Default::default()
        };
        smt.update(channel_id.to_h256(), channel.clone()).unwrap();
        assert_eq!(smt.get(&channel_id.to_h256()).unwrap(), channel);
    }
}
