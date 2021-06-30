use common::derive_more::Display;

#[derive(Clone, Debug, Display)]
pub(crate) enum RpcError {
    #[display(
        fmt = "Cannot get live cell by outpoint tx_hash {}, index {}",
        tx_hash,
        index
    )]
    CannotGetLiveCellByOutPoint { tx_hash: String, index: u32 },

    #[display(fmt = "Missing config of {:?} script", _0)]
    MissingConfig(String),

    #[display(fmt = "Ckb is not enough, address {}", _0)]
    CkbIsNotEnough(String),

    #[display(fmt = "UDT is not enough, address {}", _0)]
    UDTIsNotEnough(String),

    #[display(fmt = "UDT {} is inexistent", _0)]
    UDTInexistence(String),

    #[display(fmt = "Missing ACP cell with type_hash {}, address {}", _1, _0)]
    MissingACPCell(String, String),

    #[display(fmt = "Invalid {:?} Rpc params", _0)]
    InvalidRpcParams(String),

    #[display(fmt = "Ckb Rpc error {:?}", _0)]
    CkbClientError(String),

    #[display(fmt = "Decode json string error {:?}", _0)]
    DecodeJson(String),

    #[display(fmt = "Can not get transaction by hash {:?}", _0)]
    CannotGetTxByHash(String),

    #[display(fmt = "Can not get block by number {}", _0)]
    CannotGetBlockByNumber(u64),

    #[display(fmt = "Can not get cell by out point {:?}", _0)]
    CannotGetCellByOutPoint(String),

    #[display(fmt = "Channel error {:?}", _0)]
    ChannelError(String),

    #[display(fmt = "Invalid create account info")]
    InvalidAccountInfo,

    #[display(fmt = "Ckb transfer can only pay by from")]
    InvalidTransferPayload,
}
