use super::*;

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
pub struct TransactionTable {
    pub id: i64,
    pub tx_hash: Vec<u8>,
    pub tx_index: i32,
    pub input_count: i32,
    pub output_count: i32,
    pub block_number: i32,
    pub block_hash: Vec<u8>,
    pub tx_timestamp: i64,
    pub version: i16,
    pub cell_deps: Vec<u8>,
    pub header_deps: Vec<u8>,
    pub witnesses: Vec<u8>,
}

impl TransactionTable {
    pub fn from_view(
        view: &TransactionView,
        id: i64,
        tx_index: u32,
        block_hash: Vec<u8>,
        block_number: u64,
        tx_timestamp: u64,
    ) -> Self {
        TransactionTable {
            id,
            block_hash,
            block_number: block_number.try_into().expect("from u64"),
            tx_index: tx_index.try_into().expect("from u32"),
            tx_timestamp: tx_timestamp.try_into().expect("from u64"),
            tx_hash: view.hash().raw_data().to_vec(),
            input_count: view.inputs().len().try_into().expect("from usize"),
            output_count: view.outputs().len().try_into().expect("from usize"),
            cell_deps: view.cell_deps().as_bytes().to_vec(),
            header_deps: view.header_deps().as_bytes().to_vec(),
            witnesses: view.witnesses().as_bytes().to_vec(),
            version: view.version().try_into().expect("from u32"),
        }
    }
}
