use common::anyhow::Result;
use core_storage::{Batch, IteratorDirection, IteratorItem, Store, StoreError};

use parking_lot::RwLock;
use smallvec::SmallVec;

use std::collections::BTreeMap;
use std::sync::Arc;

pub type DBKey = SmallVec<[u8; 64]>;
pub type DBValue = Vec<u8>;

#[derive(Clone)]
pub struct DBTransaction {
    ops: Vec<DBOp>,
    db: MemoryDB,
}

#[derive(Clone, PartialEq)]
pub enum DBOp {
    Insert { key: DBKey, value: DBValue },
    Delete { key: DBKey },
}

impl DBOp {
	#[allow(dead_code)]
    pub fn key(&self) -> &[u8] {
        match *self {
            DBOp::Insert { ref key, .. } => key,
            DBOp::Delete { ref key, .. } => key,
        }
    }
}

impl DBTransaction {
    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        self.ops.push(DBOp::Insert {
            key: DBKey::from_slice(key),
            value: value.to_vec(),
        })
    }

    pub fn delete(&mut self, key: &[u8]) {
        self.ops.push(DBOp::Delete {
            key: DBKey::from_slice(key),
        });
    }
}

pub struct MemoryDB {
    inner: Arc<RwLock<BTreeMap<Vec<u8>, DBValue>>>,
}

impl Clone for MemoryDB {
    fn clone(&self) -> Self {
        MemoryDB {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Store for MemoryDB {
    type Batch = DBTransaction;

    fn new(_col: &str) -> Self {
        MemoryDB {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.inner.read().get(key.as_ref()).cloned())
    }

    fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool, StoreError> {
        Ok(self.inner.read().contains_key(key.as_ref()))
    }

    #[allow(clippy::clippy::needless_collect)]
    fn iter<K: AsRef<[u8]>>(
        &self,
        from_key: K,
        _direction: IteratorDirection,
    ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>, StoreError> {
        let db = self
            .inner
            .read()
            .clone()
            .into_iter()
            .filter(|(key, _)| key.starts_with(from_key.as_ref()))
            .map(|(k, v)| (k.into_boxed_slice(), v.into_boxed_slice()))
            .collect::<Vec<_>>();

        Ok(Box::new(db.into_iter()))
    }

    fn batch(&self) -> Result<Self::Batch, StoreError> {
        Ok(self.transaction())
    }
}

impl MemoryDB {
    pub fn create() -> Self {
        MemoryDB::new(Default::default())
    }

    pub fn display(&self) {
        let inner = self.inner.read().clone();
        println!("{:?}", inner);
    }

    fn write(&self, transaction: DBTransaction) -> Result<()> {
        let mut db = self.inner.write();
        let ops = transaction.ops;
        for op in ops {
            match op {
                DBOp::Insert { key, value } => {
                    db.insert(key.into_vec(), value);
                }
                DBOp::Delete { key } => {
                    db.remove(&*key);
                }
            }
        }
        Ok(())
    }

    fn transaction(&self) -> DBTransaction {
        DBTransaction {
            db: self.clone(),
            ops: Vec::new(),
        }
    }
}

impl Batch for DBTransaction {
    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) -> Result<(), StoreError> {
        self.put(key.as_ref(), value.as_ref());
        Ok(())
    }

    fn put_kv<K: Into<Vec<u8>>, V: Into<Vec<u8>>>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(), StoreError> {
        self.put(&key.into(), &value.into());
        Ok(())
    }

    fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), StoreError> {
        self.delete(key.as_ref());
        Ok(())
    }

    fn commit(self) -> Result<(), StoreError> {
        self.db
            .write(self.clone())
            .map_err(|e| StoreError::DBError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::random;

    fn rand_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|_| random::<u8>()).collect()
    }

    #[test]
    fn test_memory_db() {
        let (key_1, val_1) = (rand_bytes(32), rand_bytes(128));
        let (key_2, val_2) = (rand_bytes(32), rand_bytes(128));
        let (key_3, val_3) = (rand_bytes(32), rand_bytes(128));
        let (key_4, val_4) = (rand_bytes(32), rand_bytes(128));

        let db = MemoryDB::create();
        let mut batch = db.batch().unwrap();

        batch.put_kv(key_1.clone(), val_1.clone()).unwrap();
        batch.put_kv(key_2.clone(), val_2.clone()).unwrap();
        batch.put_kv(key_3.clone(), val_3.clone()).unwrap();
        batch.put_kv(key_4.clone(), val_4.clone()).unwrap();
        batch.commit().unwrap();

        assert_eq!(db.get(&key_1).unwrap().unwrap(), val_1);
        assert_eq!(db.get(&key_2).unwrap().unwrap(), val_2);
        assert_eq!(db.get(&key_3).unwrap().unwrap(), val_3);
        assert_eq!(db.get(&key_4).unwrap().unwrap(), val_4);

        let mut batch = db.batch().unwrap();
        batch.delete(&key_1);
        batch.commit().unwrap();

        assert_eq!(db.get(&key_1).unwrap(), None);
    }
}
