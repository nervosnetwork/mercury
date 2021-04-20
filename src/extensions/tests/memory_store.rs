use ckb_indexer::store::{Batch, Error as StoreError, IteratorDirection, IteratorItem, Store};
use kvdb::{DBTransaction, KeyValueDB};
use memory_db::InMemory;

use std::sync::Arc;

#[derive(Default)]
pub struct MemoryDB {
    inner: Arc<InMemory>,
    column: u32,
}

impl Clone for MemoryDB {
    fn clone(&self) -> Self {
        MemoryDB {
            inner: Arc::clone(&self.inner),
            column: self.column,
        }
    }
}

impl Store for MemoryDB {
    type Batch = MemoryDBTransaction;

    fn new(col: &str) -> Self {
        MemoryDB {
            inner: Arc::new(memory_db::create(40)),
            column: str::parse(col).unwrap(),
        }
    }

    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner
            .get(self.column, key.as_ref())
            .map_err(|e| StoreError::DBError(e.to_string()))
    }

    fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool, StoreError> {
        self.inner
            .has_key(self.column, key.as_ref())
            .map_err(|e| StoreError::DBError(e.to_string()))
    }

    fn iter<K: AsRef<[u8]>>(
        &self,
        _from_key: K,
        _direction: IteratorDirection,
    ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>, StoreError> {
        Ok(self.inner.iter(self.column))
    }

    fn batch(&self) -> Result<Self::Batch, StoreError> {
        Ok(self.transaction())
    }
}

impl MemoryDB {
    fn transaction(&self) -> MemoryDBTransaction {
        MemoryDBTransaction {
            db: Arc::clone(&self.inner),
            transaction: self.inner.transaction(),
            column: self.column,
        }
    }
}

// pub struct PrefixMemoryDB {
//     inner: Arc<InMemory>,
//     column: u32,
//     prefix: u8,
// }

// impl Clone for PrefixMemoryDB {
//     fn clone(&self) -> Self {
//         PrefixMemoryDB {
//             inner: Arc::clone(&self.inner),
//             column: self.column,
//             prefix: self.prefix,
//         }
//     }
// }

// impl Store for PrefixMemoryDB {
//     type Batch = MemoryDBTransaction;

//     fn new(col: &str) -> Self {
//         PrefixMemoryDB {
//             inner: Arc::new(InMemory::default()),
//             column: str::parse(col).unwrap(),
//             prefix: u8::default(),
//         }
//     }

//     fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, StoreError> {
//         self.inner
//             .get(self.column, key.as_ref())
//             .map_err(|e| StoreError::DBError(e.to_string()))
//     }

//     fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool, StoreError> {
//         self.inner
//             .has_key(self.column, key.as_ref())
//             .map_err(|e| StoreError::DBError(e.to_string()))
//     }

//     fn iter<K: AsRef<[u8]>>(
//         &self,
//         _from_key: K,
//         _direction: IteratorDirection,
//     ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>, StoreError> {
//         Ok(self.inner.iter(self.column))
//     }

//     fn batch(&self) -> Result<Self::Batch, StoreError> {
//         Ok(self.transaction())
//     }
// }

// impl PrefixMemoryDB {
//     fn transaction(&self) -> MemoryDBTransaction {
//         MemoryDBTransaction {
//             inner: self.inner.transaction(),
//             column: self.column,
//         }
//     }

//     fn set_prefix(mut self, prefix: u8) -> Self {
//         self.prefix = prefix;
//         self
//     }
// }

pub struct MemoryDBTransaction {
    db: Arc<InMemory>,
    transaction: DBTransaction,
    column: u32,
}

impl Batch for MemoryDBTransaction {
    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) -> Result<(), StoreError> {
        self.transaction
            .put(self.column, key.as_ref(), value.as_ref());
        Ok(())
    }

    fn put_kv<K: Into<Vec<u8>>, V: Into<Vec<u8>>>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(), StoreError> {
        self.transaction
            .put_vec(self.column, &key.into(), value.into());
        Ok(())
    }

    fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), StoreError> {
        self.transaction.delete(self.column, key.as_ref());
        Ok(())
    }

    fn commit(self) -> Result<(), StoreError> {
        self.db
            .write(self.transaction)
            .map_err(|e| StoreError::DBError(e.to_string()))
    }
}
