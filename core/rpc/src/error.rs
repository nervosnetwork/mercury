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

    #[display(fmt = "Can not get script by hash {:?}", _0)]
    CannotGetScriptByHash(String),

    #[display(fmt = "Can not get script by script hash {:?}", _0)]
    CannotGetScriptByScriptHash(String),

    #[display(fmt = "Invalid register address {:?}", _0)]
    InvalidRegisterAddress(String),

    #[display(fmt = "Invalid create account info")]
    InvalidAccountInfo,

    #[display(fmt = "Ckb transfer can only pay by from")]
    InvalidTransferPayload,

    #[display(fmt = "Can not find change cell")]
    CannotFindChangeCell,

    #[display(fmt = "Key address must be an Secp256k1 address")]
    KeyAddressIsNotSecp256k1,

    #[display(fmt = "Unsupported normal address")]
    UnsupportedNormalAddress,

    #[display(fmt = "Get Balance by block number not support yet")]
    GetBalanceByBlockNumberNotSupportYet,
}
