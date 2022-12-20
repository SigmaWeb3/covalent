use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::types::{H160, U64};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub db_path:  PathBuf,
    pub rpc_uri:  SocketAddr,
    pub address:  H160,
    pub chain_id: u64,
}

impl Config {
    pub fn chain_db_path(&self) -> PathBuf {
        let mut path_state = self.db_path.clone();
        path_state.push("rocksdb");
        path_state.push("state_data");
        path_state
    }

    pub fn trie_db_path(&self) -> PathBuf {
        let mut path_state = self.db_path.clone();
        path_state.push("rocksdb");
        path_state.push("trie_data");
        path_state
    }

    pub fn chain_id(&self) -> U64 {
        self.chain_id.into()
    }
}

pub fn parse_file<T: DeserializeOwned>(name: impl AsRef<Path>) -> Result<T> {
    let mut f = File::open(name)?;
    parse_reader(&mut f)
}

pub fn parse_reader<R: Read, T: DeserializeOwned>(r: &mut R) -> Result<T> {
    let mut buf = Vec::new();
    r.read_to_end(&mut buf)?;
    Ok(toml::from_slice(&buf)?)
}
