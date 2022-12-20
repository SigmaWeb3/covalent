use std::path::Path;

use anyhow::Result;
use bincode::serialize;
use serde::{de::DeserializeOwned, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("{0}")]
    Seld(#[from] sled::Error),
    #[error("{0}")]
    Bincode(#[from] bincode::Error),
}

#[derive(Clone)]
pub struct Store {
    db: sled::Db,
}

impl Store {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let db = sled::open(path.as_ref())?;
        let store = Self { db };

        Ok(store)
    }

    pub fn get<K: Serialize, V: DeserializeOwned>(&self, key: &K) -> Result<Option<V>, StoreError> {
        match self.db.get(&serialize(key)?)? {
            None => Ok(None),
            Some(val) => Ok(Some(bincode::deserialize(&val)?)),
        }
    }

    pub fn insert<K: Serialize, V: Serialize>(&self, key: K, val: V) -> Result<(), StoreError> {
        self.db.insert(serialize(&key)?, serialize(&val)?)?;
        Ok(())
    }

    pub fn remove<K: Serialize>(&self, key: K) -> Result<(), StoreError> {
        self.db.remove(serialize(&key)?)?;
        Ok(())
    }
}
