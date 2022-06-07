use super::*;

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
pub struct BlockTable {
    pub block_hash: Vec<u8>,
    pub block_number: i32,
    pub version: i16,
    pub compact_target: i32,
    pub block_timestamp: i64,
    pub epoch_number: i32,
    pub epoch_index: i32,
    pub epoch_length: i32,
    pub parent_hash: Vec<u8>,
    pub transactions_root: Vec<u8>,
    pub proposals_hash: Vec<u8>,
    pub uncles_hash: Vec<u8>,
    pub uncles: Vec<u8>,
    pub uncles_count: i32,
    pub dao: Vec<u8>,
    pub nonce: Vec<u8>,
    pub proposals: Vec<u8>,
}

impl From<&BlockView> for BlockTable {
    fn from(block: &BlockView) -> Self {
        let epoch = block.epoch();

        BlockTable {
            block_hash: block.hash().raw_data().to_vec(),
            block_number: block.number().try_into().expect("from u64"),
            version: block.version().try_into().expect("from u32"),
            compact_target: block.compact_target().try_into().expect("from u32"),
            block_timestamp: block.timestamp().try_into().expect("from u64"),
            epoch_number: epoch.number().try_into().expect("from u64"),
            epoch_index: epoch.index().try_into().expect("from u64"),
            epoch_length: epoch.length().try_into().expect("from u64"),
            parent_hash: block.parent_hash().raw_data().to_vec(),
            transactions_root: block.transactions_root().raw_data().to_vec(),
            proposals_hash: block.proposals_hash().raw_data().to_vec(),
            uncles_hash: block.extra_hash().raw_data().to_vec(),
            uncles: block.uncles().data().as_slice().to_vec(),
            uncles_count: block.uncle_hashes().len().try_into().expect("usize to i32"),
            dao: block.dao().raw_data().to_vec(),
            nonce: block.nonce().to_be_bytes().to_vec(),
            proposals: block.data().proposals().as_bytes().to_vec(),
        }
    }
}
