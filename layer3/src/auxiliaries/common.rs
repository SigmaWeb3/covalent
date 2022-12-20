use blake2b_ref::Blake2bBuilder;
use merkle_cbt::{merkle_tree::Merge, MerkleTree, CBMT};
use primitive_types::{H256, U256};
use serde::Serialize;

pub fn blake2b(msg: &[u8]) -> H256 {
    let mut buf = [0u8; 32];
    let mut blake2b = Blake2bBuilder::new(32).personal(b"zk pika! pi~~~").build();
    blake2b.update(msg);
    blake2b.finalize(&mut buf);
    buf.into()
}

pub trait H256Ext<H> {
    fn to_h256(&self) -> H;
}

impl H256Ext<H256> for U256 {
    fn to_h256(&self) -> H256 {
        let mut buf = [0u8; 32];
        self.to_little_endian(&mut buf);
        H256(buf)
    }
}

impl H256Ext<sparse_merkle_tree::H256> for U256 {
    fn to_h256(&self) -> sparse_merkle_tree::H256 {
        (H256Ext::<H256>::to_h256(self)).0.into()
    }
}

impl H256Ext<H256> for sparse_merkle_tree::H256 {
    fn to_h256(&self) -> H256 {
        <[u8; 32]>::from(*self).into()
    }
}

impl H256Ext<sparse_merkle_tree::H256> for H256 {
    fn to_h256(&self) -> sparse_merkle_tree::H256 {
        self.0.into()
    }
}

pub fn cbmt_merkle_root<V: Serialize>(leaves: &Vec<V>) -> H256 {
    let leaf_hashes = leaves.iter().map(|v| {
        let encoded = bincode::serialize(v).unwrap();
        blake2b(&encoded)
    });

    let tree: MerkleTree<_, MergeH256> = CBMT::build_merkle_tree(&leaf_hashes.collect::<Vec<_>>());
    tree.root()
}

struct MergeH256;

impl Merge for MergeH256 {
    type Item = H256;

    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut buf = [0u8; 32];
        let mut blake2b = Blake2bBuilder::new(32).personal(b"zk pika! pi~~~").build();

        blake2b.update(left.0.as_slice());
        blake2b.update(right.0.as_slice());
        blake2b.finalize(&mut buf);

        buf.into()
    }
}
