use crate::error::MercuryError;

use anyhow::Result;
use ckb_indexer::store::{Batch, Error, IteratorDirection, IteratorItem, Store};
use ckb_types::bytes::Bytes;

use std::sync::{Arc, RwLock};

pub struct PrefixStore<S> {
    store: S,
    prefix: Bytes,
}

impl<S> PrefixStore<S> {
    pub fn new_with_prefix(store: S, prefix: Bytes) -> Self {
        Self { store, prefix }
    }
}

impl<S: Store> Store for PrefixStore<S> {
    type Batch = PrefixStoreBatch<S::Batch>;

    fn new(path: &str) -> Self {
        Self::new_with_prefix(S::new(path), Bytes::new())
    }

    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.store.get(add_prefix(&self.prefix, key))
    }

    fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool, Error> {
        self.store.exists(add_prefix(&self.prefix, key))
    }

    fn iter<K: AsRef<[u8]>>(
        &self,
        from_key: K,
        direction: IteratorDirection,
    ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>, Error> {
        self.store
            .iter(add_prefix(&self.prefix, from_key), direction)
    }

    fn batch(&self) -> Result<Self::Batch, Error> {
        let inner_batch = self.store.batch()?;
        Ok(PrefixStoreBatch::new(inner_batch, self.prefix.clone()))
    }
}

pub struct PrefixStoreBatch<B> {
    batch: B,
    prefix: Bytes,
}

impl<B> PrefixStoreBatch<B> {
    pub fn new(batch: B, prefix: Bytes) -> Self {
        Self { batch, prefix }
    }
}

fn add_prefix<P: AsRef<[u8]>, K: AsRef<[u8]>>(prefix: P, key: K) -> Vec<u8> {
    let mut result = vec![];
    result.extend_from_slice(prefix.as_ref());
    result.extend_from_slice(key.as_ref());
    result
}

impl<B: Batch> Batch for PrefixStoreBatch<B> {
    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) -> Result<(), Error> {
        self.batch.put(add_prefix(&self.prefix, key), value)
    }

    fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), Error> {
        self.batch.delete(add_prefix(&self.prefix, key))
    }

    fn commit(self) -> Result<(), Error> {
        self.batch.commit()
    }
}

pub struct BatchStore<S: Store> {
    store: S,
    batch: Arc<RwLock<Option<S::Batch>>>,
}

impl<S: Clone + Store> Clone for BatchStore<S> {
    fn clone(&self) -> Self {
        BatchStore {
            store: self.store.clone(),
            batch: Arc::clone(&self.batch),
        }
    }
}

impl<S: Store> BatchStore<S> {
    pub fn create(store: S) -> Result<Self> {
        let batch = store.batch()?;
        Ok(Self {
            store,
            batch: Arc::new(RwLock::new(Some(batch))),
        })
    }

    pub fn commit(self) -> Result<S> {
        let mut batch = self.batch.write().expect("poisoned");
        if batch.is_none() {
            return Err(MercuryError::DBError("Someone still holds the batch!".to_string()).into());
        }
        batch.take().unwrap().commit()?;
        Ok(self.store)
    }
}

impl<S: Store> Store for BatchStore<S> {
    type Batch = BatchStoreBatch<S::Batch>;

    fn new(path: &str) -> Self {
        Self::create(S::new(path)).expect("new store failure")
    }

    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.store.get(key)
    }

    fn exists<K: AsRef<[u8]>>(&self, key: K) -> Result<bool, Error> {
        self.store.exists(key)
    }

    fn iter<K: AsRef<[u8]>>(
        &self,
        from_key: K,
        direction: IteratorDirection,
    ) -> Result<Box<dyn Iterator<Item = IteratorItem> + '_>, Error> {
        self.store.iter(from_key, direction)
    }

    fn batch(&self) -> Result<Self::Batch, Error> {
        let batch = {
            let mut batch = self.batch.write().expect("poisoned");
            if batch.is_none() {
                return Err(Error::DBError("Someone still holds the batch!".to_string()));
            }
            batch.take().unwrap()
        };
        Ok(BatchStoreBatch {
            holder: Arc::clone(&self.batch),
            batch,
        })
    }
}

pub struct BatchStoreBatch<B> {
    holder: Arc<RwLock<Option<B>>>,
    batch: B,
}

impl<B> Batch for BatchStoreBatch<B>
where
    B: Batch,
{
    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: K, value: V) -> Result<(), Error> {
        self.batch.put(key, value)
    }

    fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), Error> {
        self.batch.delete(key)
    }

    fn commit(self) -> Result<(), Error> {
        let mut batch = self.holder.write().expect("poisoned");
        batch.replace(self.batch);
        Ok(())
    }
}
