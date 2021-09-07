use common::Result;
use db_protocol::{IteratorItem, KVStore, KVStoreBatch};
use db_rocksdb::rocksdb::{DBIterator, IteratorMode};
use db_rocksdb::{RocksdbBatch, RocksdbStore};

use ckb_types::bytes::Bytes;

#[derive(Clone)]
pub struct PrefixKVStore {
    store: RocksdbStore,
    prefix: Bytes,
}

impl PrefixKVStore {
    pub fn new_with_prefix(store: RocksdbStore, prefix: Bytes) -> Self {
        PrefixKVStore { store, prefix }
    }

    pub fn snapshot_iter(&self, mode: IteratorMode) -> Box<DBIterator> {
        Box::new(self.store.inner().snapshot().iterator(mode))
    }
}

impl KVStore for PrefixKVStore {
    type Batch = PrefixKVStoreBatch;

    fn new(path: &str) -> Self {
        Self::new_with_prefix(RocksdbStore::new(path), Bytes::new())
    }

    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>> {
        self.store.get(add_prefix(&self.prefix, key))
    }

    fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool> {
        self.store.exists(add_prefix(&self.prefix, key))
    }

    fn iter<K: AsRef<[u8]>>(
        &self,
        from_key: K,
        direction: db_protocol::IteratorDirection,
    ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>> {
        self.store
            .iter(add_prefix(&self.prefix, from_key), direction)
    }

    fn batch(&self) -> Result<Self::Batch> {
        let inner_batch = self.store.batch()?;
        Ok(PrefixKVStoreBatch::new(inner_batch, self.prefix.clone()))
    }
}

pub struct PrefixKVStoreBatch {
    batch: RocksdbBatch,
    prefix: Bytes,
}

impl KVStoreBatch for PrefixKVStoreBatch {
    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) -> Result<()> {
        self.batch.put(add_prefix(&self.prefix, key), value)
    }

    fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<()> {
        self.batch.delete(add_prefix(&self.prefix, key))
    }

    fn commit(self) -> Result<()> {
        self.batch.commit()
    }
}

impl PrefixKVStoreBatch {
    pub fn new(batch: RocksdbBatch, prefix: Bytes) -> Self {
        PrefixKVStoreBatch { batch, prefix }
    }
}

pub fn add_prefix<P: AsRef<[u8]>, K: AsRef<[u8]>>(prefix: P, key: K) -> Vec<u8> {
    let mut result = vec![];
    result.extend_from_slice(prefix.as_ref());
    result.extend_from_slice(key.as_ref());
    result
}
