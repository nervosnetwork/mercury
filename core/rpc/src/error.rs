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

    #[display(
        fmt = "Ckb is not enough, require {} unconstrained ckb, short {} unconstrained ckb",
        _0,
        _1
    )]
    CkbIsNotEnough(String, String),

    #[display(fmt = "UDT is not enough, require {} udt, short {} udt", _0, _1)]
    UDTIsNotEnough(String, String),

    #[display(fmt = "UDT {} is in existent", _0)]
    UDTInExistence(String),

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

    #[display(fmt = "Invalid address {:?}", _0)]
    InvalidAddress(String),

    #[display(fmt = "Invalid normal address {:?}", _0)]
    InvalidNormalAddress(String),

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

    #[display(fmt = "The from normal address in transfer payload is mixed")]
    FromNormalAddressIsMixed,

    #[display(fmt = "Unsupported source")]
    UnsupportedSource,

    #[display(fmt = "Unsupported action")]
    UnsupportedAction,

    #[display(fmt = "FeePaidBy address doesn't have enough capacity")]
    FeePaidByAddressInsufficientCapacity,

    #[display(fmt = "Asset account not support for CKB yet")]
    CkbAssetAccountNotSupport,

    #[display(fmt = "At least one udt_hash is needed")]
    AtLeastOneUDTHashIsNeeded,

    #[display(fmt = "Exceed the maximum item number")]
    ExceedMaxItemNum,

    #[display(fmt = "Fee Address can not be contained in From Addresses")]
    InValidFeeAddress,

    #[display(fmt = "No assets for collection")]
    NoAssetsForCollection,

    #[display(fmt = "Script type in cheque not support")]
    UnSupportScriptTypeForCheque,
}
