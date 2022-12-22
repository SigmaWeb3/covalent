use primitive_types::{H160, H256};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    generate_keypair,
    rand::rngs::OsRng,
    Message, PublicKey, Secp256k1, SecretKey,
};
use sha3::{Digest, Keccak256};

use crate::types::{RawTransaction, SignedTransaction};

use super::common::Hash;

#[allow(dead_code)]
#[derive(Clone)]
pub struct Wallet {
    sk: SecretKey,
    pk: PublicKey,
    addr: H160,
}

impl Wallet {
    pub fn random() -> Self {
        let (sk, pk) = generate_keypair(&mut OsRng);
        let addr = Self::address(&pk);
        Self { sk, pk, addr }
    }

    pub fn addr(&self) -> H160 {
        self.addr
    }

    pub fn sign(&self, msg: H256) -> Result<[u8; 65], secp256k1::Error> {
        let msg = Message::from_slice(&msg.0)?;
        let secp = Secp256k1::new();
        let (rec_id, bytes) = secp
            .sign_ecdsa_recoverable(&msg, &self.sk)
            .serialize_compact();

        let mut buf = [0u8; 65];
        buf[..64].copy_from_slice(&bytes);
        buf[64] = rec_id.to_i32().try_into().unwrap();

        Ok(buf)
    }

    pub fn sign_tx(&self, raw_tx: RawTransaction) -> Result<SignedTransaction, secp256k1::Error> {
        let tx_hash = raw_tx.hash();
        let sig = self.sign(tx_hash)?;
        let signed_tx = SignedTransaction {
            raw: raw_tx,
            sig: sig.to_vec(),
            from: self.addr(),
            hash: tx_hash,
        };

        Ok(signed_tx)
    }

    pub fn address(pk: &PublicKey) -> H160 {
        let mut hasher = Keccak256::new();
        hasher.update(&pk.serialize_uncompressed()[1..]);
        H160::from_slice(&hasher.finalize()[12..])
    }

    pub fn recover_address(msg: H256, sig: [u8; 65]) -> Result<H160, secp256k1::Error> {
        let msg = Message::from_slice(&msg.0)?;

        let rec_id = extract_rec_id(sig[64])?;
        let rec_sig = RecoverableSignature::from_compact(&sig[..64], rec_id)?;

        let secp = Secp256k1::new();
        let pk = secp.recover_ecdsa(&msg, &rec_sig)?;

        Ok(Self::address(&pk))
    }
}

fn extract_rec_id(rec_id: u8) -> Result<RecoveryId, secp256k1::Error> {
    let param = match rec_id {
        r if r == 27 => 0,
        r if r == 28 => 1,
        r => r,
    };
    Ok(RecoveryId::from_i32(param.into())?)
}
