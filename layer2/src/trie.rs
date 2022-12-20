use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use sled::{Db, Error};

pub struct RocksTrieDB {
    db: Arc<Db>,
}

impl RocksTrieDB {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        RocksTrieDB {
            db: Arc::new(sled::open(path).expect("open")),
        }
    }
}

impl cita_trie::DB for RocksTrieDB {
    type Error = Error;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.db.get(key)?.map(|inner| (*inner).to_vec()))
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        self.db.contains_key(key)
    }

    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Self::Error> {
        let _ = self.db.insert(key, value)?;
        Ok(())
    }

    fn insert_batch(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        self.db
            .transaction::<_, _, Error>(|tx_db| {
                for (k, v) in keys.iter().zip(values.iter()) {
                    tx_db.insert(k.clone(), v.clone())?;
                }
                Ok(())
            })
            .unwrap();
        Ok(())
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        let _ = self.db.remove(key)?;
        Ok(())
    }

    fn remove_batch(&self, keys: &[Vec<u8>]) -> Result<(), Self::Error> {
        self.db
            .transaction::<_, _, Error>(|tx_db| {
                for k in keys.iter() {
                    let _ = tx_db.remove(k.clone())?;
                }
                Ok(())
            })
            .unwrap();

        Ok(())
    }

    fn flush(&self) -> Result<(), Self::Error> {
        let _ = self.db.flush()?;
        Ok(())
    }
}
