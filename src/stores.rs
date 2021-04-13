use ckb_indexer::store::{Batch, Error, IteratorDirection, IteratorItem, Store};
use ckb_types::bytes::Bytes;

pub struct PrefixStore<S> {
    store: S,
    prefix: Bytes,
}

impl<S> PrefixStore<S> {
    pub fn new_with_prefix(store: S, prefix: Bytes) -> Self {
        Self { store, prefix }
    }
}

impl<S> Store for PrefixStore<S>
where
    S: Store,
{
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
        Ok(PrefixStoreBatch {
            batch: inner_batch,
            prefix: self.prefix.clone(),
        })
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

impl<B> Batch for PrefixStoreBatch<B>
where
    B: Batch,
{
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
