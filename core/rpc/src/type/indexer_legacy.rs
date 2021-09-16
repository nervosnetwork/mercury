#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CellOutputWithOutPoint {
    pub out_point: OutPoint,
    pub block_hash: H256,
    pub capacity: Capacity,
    pub lock: Script,
    #[serde(rename = "type")]
    pub type_: Option<Script>,
    pub output_data_len: Uint64,
    pub cellbase: bool,
}
