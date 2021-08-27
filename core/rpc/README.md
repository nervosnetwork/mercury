# Mercury JSON-RPC Protocols

- [Major Changes Compared to Version 0.1.0](#major-changes-compared-to-version-010)
- [Core Concept](#core-concept)
  - [Identity](#identity)
  - [Address](#address)
  - [Balance Type](#balance-type)
  - [Source](#source)
  - [Double-entry Style Structure](#double-entry-style-structure)
  - [Error Code](#error-code)
- [RPC Methods](#rpc-methods)
  - [Method `get_balance`](#method-getbalance)
  - [Method `get_block_info`](#method-getblockinfo)
  - [Method `get_transaction_info`](#method-gettransactioninfo)
  - [Method `query_transactions`](#method-querytransactions)
  - [Method `build_adjust_account_transaction`](#method-buildadjustaccounttransaction)
  - [Method `build_transfer_transaction`](#method-buildtransfertransaction)
  - [Method `build_smart_transfer_transaction`](#method-buildsmarttransfertransaction)
  - [Method `register_addresses`](#method-registeraddresses)
  - [Method `build_deposit_transaction`](#method-builddeposittransaction)
  - [Method `build_withdraw_transaction`](#method-buildwithdrawtransaction)
  - [Method `get_spent_transaction`](#method-getspenttransaction)
  - [Method `advance_query`](#method-advancequery)
  - [Method `get_mercury_info`](#method-getmercuryinfo)
  - [Method `get_db_info`](#method-getdbinfo)
- [RPC Types](#rpc-types)
  - [Type `Identity`](#type-identity)
  - [Type `Address`](#type-address)
  - [Type `RecordId`](#type-recordid)
  - [Type `AssetInfo`](#type-assetinfo)
  - [Type `Balance`](#type-balance)
  - [Type `BlockRange`](#type-blockrange)
  - [Type `PaginationRequest`](#type-paginationrequest)
  - [Type `BlockInfo`](#type-blockinfo)
  - [Type `TransactionInfo`](#type-transactioninfo)
  - [Type `Record`](#type-record)
  - [Type `Claimable`](#type-claimable)
  - [Type `Fixed`](#type-fixed)
  - [Type `DaoInfo`](#type-daoinfo)
  - [Type `Deposit`](#type-deposit)
  - [Type `Withdraw`](#type-withdraw)
  - [Type `BurnInfo`](#type-burninfo)
  - [Type `SignatureEntry`](#type-signatureentry)
  - [Type `From`](#type-from)
  - [Type `To`](#type-to)
  - [Type `SinceConfig`](#type-sinceconfig)
  - [Type `SmartTo`](#type-smartto)
  - [Type `ScriptWrapper`](#type-scriptwrapper)
  - [Type `CellInfo`](#type-cellinfo)
  - [Type `MercuryInfo`](#type-mercuryinfo)
  - [Type `Extension`](#type-extension)
  - [Type `DBInfo`](#type-dbinfo)

## Major Changes Compared to [Version 0.1.0](https://github.com/nervosnetwork/mercury/blob/v0.1.0-rc.3/core/rpc/README.md)

- Optimize core concepts design by using identity instead of key addresses.
- Align the balance type with CKB.
- Make the core terms more accurate.
- Optimize the design of the original interfaces.
- Support DAO query, deposit and withdraw operations.
- Support to get a spent transaction according to the outpoint.
- Add an interface for adjusting the number of ACP cells (creating and destroying cells are automatically inferred).
- Support advanced queries for Lumos.
- Improve error code design to be more friendly for applications.

## Core Concept

Before exploring the Mercury interfaces, it is crucial to understand Mercury's core concepts.

### Identity

The CKB [Cell Model](https://docs.nervos.org/docs/basics/concepts/cell-model) is similar to that of [UTXO](https://en.wikipedia.org/wiki/Unspent_transaction_output) in Bitcoin's terminology. Cell is the basic unit in CKB. The full set of unspent cells in CKB is considered being the full state of CKB at that particular point in time. A [lock script](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#42-cell) defines the ownership of a cell.

Identity is an abstract concept used to manage lock scripts of the same ownership. For example, an secp256k1 private key can unlock many lock scripts including [secp256k1/blake160](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#42-cell), [acp](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0026-anyone-can-pay/0026-anyone-can-pay.md) and [cheque](https://talk.nervos.org/t/sudt-cheque-deposit-design-and-implementation/5209). The secp256k1 public key corresponding to this private key can act as the identity of these lock scripts.

The structure of identity is `<1 byte flag> <20 bytes identity content>`. Identity is also used in [RCE](https://talk.nervos.org/t/rfc-regulation-compliance-lock/5788) and [pw-core](https://github.com/lay2dev/pw-core).

- `flag`: The flexible nature of CKB lock script design enables any public chains' signature algorithm to be implemented on CKB. For example, the community's pw-core project supports both the signature algorithms of BTC, ETH, EOS, TRON, and Doge etc. The flag is used to distinguish the public keys of these different algorithms. Identity can also support complex contracts that cannot use a single public key to identify the ownership.

- `content`: If a flag represents a public key, then the content is the blake160 hash of the public key. Otherwise, the content is the blake160 hash of the lock.

### Address

Currently, Mercury only support [ckb address format](https://github.com/nervosnetwork/rfcs/tree/master/rfcs/0021-ckb-address-format) which is encoded from lock script. In the near future, Mercury will also support address specifications of some other public chains like BTC, ETH, EOS, TRON, and Doge etc. The support for other addresses is to be implemented when Mercury supports pw-core.

### Mode

The native token for the Nervos CKB is CKByte which is also short as CKB, and custom token standards such as [sUDT](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0025-simple-udt/0025-simple-udt.md) (simple User-Defined Token) and [xUDT](https://talk.nervos.org/t/rfc-extensible-udt/5337) (Extensible User-Defined Token) is also supported. Through these standards, anyone can create and issue custom tokens on CKB.

CKB solves the problem of [state explosion](https://medium.com/@happypeter1983/what-is-blockchain-state-explosion-22dd531eeb21) through a unique [economic model](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#5-economic-model) design. In this design, CKByte is not only play roles of asset but also make capacity. However, udt assets can not make capacity, they can only be exist along with a certain amount of CKBytes.

The mode is used to specify who provides CKBytes to make capacity for the output cell of recipients in a transfer.

- HoldByFrom: sender provides CKBytes for the output cell of recipients.

- HoldByTo: receipient provides CKBytes for the output cell of himself.

### Balance Type

- free - The assets that are freely spendable.
- occupied: The CKB that is used to provide capacity.
- freezed: The assets that are locked, such as cellbase, dao, etc. that have not been unlocked.
- claimbale: The UDT assets on the cheque cell that are unclaimed and not timed out.

### Source

Only two categories of balance are spendable, they are free and claimable which can be used as source in a transfer.

### Double-entry Style Structure

Mercury has a double-entry style blockchain data structure ([`BlockIfo`](#type-blockinfo) -> [`TransactionInfo`](#type-transactioninfo) -> [`Record`](#type-record)) that is abstracted on top of the CKB data structure. The `Record` is designed to reflect the changes in the asset amount of an address in a transaction.

### Error Code

Mercury, as the middle layer to the application layer, must provide fixed error codes that are convenient for the application to handle.

- The general error code is -1 ~ -999.
- The error code range of the ckb-rpc interface is -1000 ~ -2999.
- The error code range of Mercury is -10000 ~ -12999
- RPC error codes is pre-defined [here](https://github.com/nervosnetwork/ckb/blob/cef2a32d31db8cfe73c634f7f1c52b86c4a8f404/rpc/src/error.rs#L15).

## RPC Methods

### Method `get_balance`

- `get_balance(item, asset_types, tip_block_number)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `asset_types`: `Array<`[`AssetInfo>`](#type-assetinfo)`>`
  - `tip_block_number`: `Uint64|null`
- result
  - `tip_block_number`: `Uint64`
  - `balances`: `Array<`[`Balance`](#type-balance)`>`

Return the balance of specified assets of the given item.

Params

- `item` - Specify the object used to get balance of.
  - If specify an identity, the balance of addresses controlled by the identity will be accumulated.
  - If specify an address, the balance of unspent records of the address will be accumulated.  
  - If specify the id of an unspent record, the balance of the record will be returned.
- `tip_block_number` - Specify a block of giving block_number as the tip of the blockchain for querying。
  - If tip_block_number is null, the querying is based on the latest blockchain.
  - If tip_block_number is not null, the querying is based on the historical blockchain with the giving tip.
- `asset_types` - Specify a set of asset types for querying.
  - If the set is empty, it will return the balance of any asset matching the querying.

Returns

- `tip_block_number` - Show the tip of the blockchain for querying.
- `balances` - Show a list of balance information matching the querying.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {

    }
  ]
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {

  }
}
```

### Method `get_block_info`

- `get_block_info(block_number, block_hash)`
  - `block_number`: `Uint64|null`
  - `block_hash`: `string|null`
- result
  Return the [`BlockInfo`](#type-blockinfo) of the specified block.

Return the double-entry style block structure of a specified block.

Params

- `block_number` - Specify the block number for querying.
- `block_hash` - Specify the block hash for querying.

Returns

- If both `block_number` and `block_hash` are `null`, return the latest block.
- If `block_number` is `null` and `block_hash` is not `null`, return the block matches `block_hash`.
- If `block_number` is not `null` and `block_hash` is `null`, return the block on the canonical chain matches `block_number`.
- If both `block_number` and `block_hash` are not `null`, return the block on the canonical chain both matches `block_number` and `block_hash`.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_block_info",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
  
  }
}
```

### Method `get_transaction_info`

- `get_transaction_info(tx_hash)`
  - `tx_hash`: `string`
- result
  - `transaction`: [`TransactionInfo`](#type-transactioninfo)`|null`
  - `status`: `"pending"|"proposed"|"committed"|"Rejected"|"Unknown"`
  - `reject_reason`: `Uint8 |null`
  - `block_hash`: `string|null`
  - `block_number`: `Uint64|null`

Return the double-entry style transaction along with the status of a giving transaction hash.

Params

- `tx_hash` - Specify the transaction hash for querying.

Returns

- `transaction` - double-entry style transaction of the giving `tx_hash`.
- `status`
  - Status "pending" means the transaction is in the pool and not proposed yet.
  - Status "proposed" means the transaction is in the pool and has been proposed.
  - Status "committed" means the transaction has been committed to the canonical chain.
  - Status "Rejected" means the transaction has been rejected by the pool.
  - Status "Unknown" means the transaction was unknown for the pool.
- `reject_reason` - If the transaction is "Rejected", it will return the code of the rejection reason.
- `block_hash` - If the transaction is "committed", it will return the block hash of the involving block.
- `block_number` - If the transaction is "committed", it will return the block number of the involving block.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_transaction_info",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {

  }
}
```

### Method `query_transactions`

- `query_transactions(item, asset_types, extra_filter, block_range, pagination, structure_type)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `asset_types`: `Array<`[`AssetInfo>`](#type-assetinfo)`>`
  - `extra_filter`: `"DAO"|"Cellbase" |null`
  - `block_range`: [`BlockRange`](#type-blockrange)`|null`
  - `pagination`: [`PaginationRequest`](#type-paginationrequest)`|null`
  - `structure_type`: `"Native"|"DoubleEntry"`
- result
  - `response`: `Array<`[`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithStatus`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionwithstatus)`>`
  - `next_cursor`: `Int64|null`
  - `total_count`: `Uint64|null`

Return generic transactions and pagination settings from practical searching.

Params

- `item` - Specify the object used to query transactions involved.
  - If specify an identity, the transactions involve addresses controlled by the identity will be returned.
  - If specify an address, the transactions involve records of the address will be returned.  
  - If specify the id of a record, the transaction involve the record will be returned.
- `asset_types` - Specify a set of asset types for querying.
  - If the set is empty, it will return transactions involve any asset matching the querying.
- `extra_filter` - Specify the filter applying to the querying.
  - If set null, it will not apply extra filter.
- `block_range` - Specify the block range for querying.
- `pagination` - Specify the pagination set.
  - If set null, no pagination set will be applied.
- `structure_type` - Specify the structure type of returning transactions.
  - If set Native, it will return CKB native structure of transaction.
  - If set DoubleEntry, it will return double-entry style structure of transaction.

Returns

- `response` - Return a list of transactions meets the querying.
- `next_cursor` - Return the beginning cursor for next querying.
  - If return null, it means there's no further transactions matching the querying.
- `total_count` - Return the total count of transactions matching the querying ignore pagination set. It can be used for calculating total pages.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_transactions",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {

  }
}
```

### Method `build_adjust_account_transaction`

- `build_adjust_account_transaction(item, from, asset_type, account_number, extra_ckb, fee_rate)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `from`: `Array<`[`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)`>`
  - `asset_type`: [`AssetInfo`](#type-assetinfo)
  - `account_number`: `Uint32|null`
  - `extra_ckb`: `Uint64|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)`|null`
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

In CKB, users should create asset accounts for receiving UDT asset. Each account is binding to only one type of asset and occupies 142 CKB. Normally, a user create one account for one asset is enough. But sometimes, business users need multiple accounts to receive multiple transfers at the same time. So, this interface is designed to controll the quantity of accounts.

- If the number of existing accounts below required number, it will build transactions to create accounts.
- If the number of existing accounts above required number, it will build transactions to recycle accounts. The udt assets in the recycled account will move to the remained account.
  - If udt assets is not zero, required number can not be zero, because we need at least one account to place the udt assets.
- If the number of existing accounts equal required number, it will build no transaction.

Params

- `item` - Specify the object used to create/recycle account upon.
  - If specify an identity, the account controlled by the identity will be create/recycle.
  - If specify an address, the account controlled by the identity behind the address will be create/recycle.
  - If specify the id of a record, the account controlled by the identity behind the record will be create/recycle.
- `from` - Specify the object used to provide CKB for creating asset accounts.
  - If set null, it will get CKB from `item`.
- `asset_type` - Specify a kind of asset for creating asset accounts.
- `account_number` - Specify a target number of account.
- `extra_ckb` - Specify the amount of extra CKB inject into account for paying fees or other usage in the future.
- `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.

Returns

- `tx_view` - The raw transaction of creating/recycling account.
- `signature_entries` - Signature entries for signing.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_adjust_account_transaction",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    
  }
}
```

### Method `build_transfer_transaction`

- `build_transfer_transaction(asset_type, from, to, change, fee_rate, since)`
  - `asset_type`: [`AssetInfo`](#type-assetinfo)
  - `from`: `Array<`[`From`](#type-from)`>`
  - `to`: `Array<`[`To`](#type-to)`>`
  - `change`: `string|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

Build a raw transfer transaction and signature entries for signing.

Params

- `asset_type` - Specify the kind of asset to transfer.
- `from` - Specify who pays assets.
- `to` - Specify receipient's address, amount, etc.
- `change` - Specify an address for change.
  - If set null, the 1st item in `from` will perform the change address.
- `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.
- `since` - Specify since configuration to prevent the transaction to be spent before a certain block timestamp or a block number.

Returns

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_transfer_transaction",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `build_smart_transfer_transaction`

- `build_smart_transfer_transaction(asset_type, from, to, change, fee_rate, since)`
  - `asset_type`: [`AssetInfo`](#type-assetinfo)
  - `from`: `Array<`[`Address`](#type-address)`>`
  - `to`: `Array<`[`SmartTo`](#type-smartto)`>`
  - `change`: `string|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

The difference between `build_smart_transfer_transaction` and `build_transfer_transaction` is that `build_smart_transfer_transaction` follows a smart strategy to infer `source` and `mode`.

Params

- `asset_type` - Specify the kind of asset to transfer.
- `from` - Specify who pays assets.
- `to` - Specify receipient's address and amount.
- `change` - Specify an address for change.
  - If set null, the 1st address in `from` will perform the change address.
- `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.
- `since` - Specify since configuration to prevent the transaction to be spent before a certain block timestamp or a block number.

Returns

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_smart_transfer_transaction",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `register_addresses`

- `register_addresses(addresses)`
  - `addresses`: `Array<`[`Address`](#type-address)`>`
- result
  Return a list of lock script hash of the register addresses.

Register addresses are used for mercury to reveal the receivers' addresses of a cheque.

Params

- `addresses` - Addresses for register.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "register_addresses",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {

  }
}
```

### Method `build_deposit_transaction`

- `build_deposit_transaction(from, to, amount, fee_rate)`
  - `from`: `Array<`[`From`](#type-from)`>`
  - `to`: [`Address`](#type-address)`|null`
  - `amount`: `Uint64`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

Build a transaction to deposit specified amount of CKB to Dao.

Params

- `from` - Specify from where to collect CKB for Dao deposition.
- `to` - Specify the CKB would deposit on which address.
  - If set null, the CKB will deposit on the address in `from`.
- `amount` - Specify the amount of CKB for deposition.
- `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.

Returns

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_deposit_transaction",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `build_withdraw_transaction`

- `build_withdraw_transaction(from, pay_fee, fee_rate)`
  - `from`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `pay_fee`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)`|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

Build a transaction to withraw specified deposits from Dao.

Params

- `from` - Specify where to get deposit cells.
- `pay_fee` - Specify who pays the fee.
  - If set null, the address in `from` will pay the fee.
- `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.

Returns

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_withdraw_transaction",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `get_spent_transaction`

- `get_spent_transaction(outpoint, view_type)`
  - `outpoint`: [`OutPoint`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-outpoint)
  - `view_type`: `"TransactionView"|"TransactionInfo"`
- result
  - `transaction`: [`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithStatus`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionwithstatus)

Get the transaction which use the specified outpoint as a input.

Params

- `outpoint` - Specify the outpoint for querying.
- `view_type` - Specify the structure type of the returning transaction.

Returns

- `transaction` - The spent transaction returned.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_spent_transaction",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `advance_query`

- `advance_query(lock, type, data, args_len, block_range, pagination, query_type)`
  - `lock`: [`ScriptWrapper`](#type-scriptwrapper)`|null`
  - `type`: [`ScriptWrapper`](#type-scriptwrapper)`|null`
  - `data`: `string|null`
  - `args_len`: `Uint32|null`
  - `block_range`: [`BlockRange`](#type-blockrange)`|null`
  - `pagination`: [`PaginationRequest`](#type-paginationrequest)
  - `query_type`: `"Cell"|"Transaction"`
- result
  - `result`: `Array<`[`CellInfo`](#type-cellinfo)`|`[`TransactionWithStatus`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionwithstatus)`>`
  - `next_cursor`: `Int64|null`
  - `total_count`: `Uint64|null`

Advance query for searching cells or transactions.

Params

- `lock` - Specify the lock info for querying.
- `type` - Specify the type info for querying.
- `data` - Specify the data for querying.
- `args_len` - Specify args length for querying.
- `block_range` - Specify the range of block for querying.
- `pagination` - Specify the pagination configuration fot querying.
- `query_type` - Specify the returning type.

Returns

- `result` - The results matching the querying.
- `next_cursor` - Return the beginning cursor for next querying.
  - If return null, it means there's no further transactions matching the querying.
- `total_count` - Return the total count of data matching the querying ignore pagination set. It can be used for calculating total pages.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "advance_query",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `get_mercury_info`

- `get_mercury_info()`
- result
  - `mercury_info`: [`MercuryInfo`](#type-mercuryinfo)

Return info of mercury.

Returns

- `mercury_info` - The info of mercury.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_mercury_info",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

### Method `get_db_info`

- `get_db_info()`
- result
  - `db_info`: [`DBInfo`](#type-dbinfo)

Return info of database.

Returns

- `db_info` - The info of database.

Examples

- Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_db_info",
  "params": {

  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
   
  }
}
```

## RPC Types

### Type `Identity`

Fields

- `identity`: `string` - Specify an identity.

### Type `Address`

Fields

- `address`: `string` - Specify an address.

### Type `RecordId`

Fields

- `record_id`: `string` - Specify the id of a record.

### Type `AssetInfo`

Fields

- `asset_type`: `"CKB"|"UDT"` - Specify the type of an asset.
- `udt_hash`: `string` - Specify the hash of an udt asset.

### Type `Balance`

Fields

- `address`: `string` - Specify the address of the balance belongs to.
- `asset_type`: [`AssetInfo`](#type-assetinfo) - Specify the asset type of the balance.
- `free`: `BigInt` - Specify the amount of assets that are freely spendable.
- `occupied`: `Uint64` - Specify the amount of CKB that is used to provide capacity.
- `freezed`: `BigInt` - Specify the amount of assets that are locked.
- `claimable`: `BigInt` - Specify the amount of UDT assets on the cheque cell that are unclaimed and not timed out.

### Type `BlockRange`

Fields

- `from`: `Uint64` - Specify the start block number of the range.
- `to`: `Uint64` - Specify the end block number of the range.

### Type `PaginationRequest`

This is a cursor-based pagination configuration.

Fields

- `cursor`: `Int64|null` - Specify the beginning cursor for querying.
  - If set null, the querying starts from the biggest cursor when order is desc and from the smallest cursor when order is asc.
- `order`: `"Asc"|"Desc"` - Specify the order of the returning data.
- `limit`: `Uint64|null` - Specify the entry limit per page of the querying.
  - If set null, it will set a default limit, such as 50.
- `total_count`: `bool` - Specify whether return the total count.

### Type `BlockInfo`

A double-entry style blockchain structure.

Fields

- `block_number`: `Uint64` - Specify the block number.
- `block_hash`: `string` - Specify the block hash.
- `parent_block_hash`: `string` - Specify the parent block hash.
- `timestamp`: `Uint64` - Specify the timestamp.
- `transactions`: `Array<`[`TransactionInfo`](#type-transactioninfo)`>` - Specify double-entry style transactions in the block.

### Type `TransactionInfo`

A double-entry style transaction structure.

Fields

- `tx_hash`: `string` - Specify the transaction hash.
- `records`: `Array<`[`Record`](#type-record)`>` - Specify records in the transaction.
- `fee`: `Uint64` - Specify fee paied by this transaction.
- `burn`: `Array<`[`BurnInfo`](#type-burninfo)`>` - Specify the amount of UDT asset burned in this transaction.

### Type `Record`

A double-entry style structure which is designed to reflect the changes in the asset amount of an address in a transaction.

Fields

- `id`: `string` - Specify the identify of the record.
- `address`: `string` - Specify the address of which amounts changed.
- `amount`: `BigInt` - Specify the amount changes.
  - The value is negative when the record is spent and positive when the record is new.
- `asset_type`: [`AssetInfo`](#type-assetinfo) - Specify the asset type of the record.
- `status`: [`Claimable`](#type-claimable)`|`[`Fixed`](#type-fixed) - Specify the status of the record.
- `extra`: [`DaoInfo`](#type-daoinfo)`| "Cellbase"|null` - Specify the extra info of the record.

### Type `Claimable`

Fields

- `block_number`: `Uint64` - Specify the number of block which contains a cheque creation transaction.

### Type `Fixed`

Fields

- `block_number`: `Uint64` - Specify the number of block which contains a transaction fixed this record.

### Type `DaoInfo`

Fields

- `state`: [`Deposit`](#type-deposit)`|`[`Withdraw`](#type-withdraw) - Specify the state of a dao operation.
- `reward`: `Uint64` - Specify the accumulate reward of a dao operation.

### Type `Deposit`

Fields

- `block_number`: `Uint64` - Specify the number of block which contains a dao deposit transaction.

### Type `Withdraw`

Fields

- `block_number`: `Uint64` - Specify the number of block which contains a dao withdraw transaction.

### Type `BurnInfo`

Fields

- `udt_hash`: `string` - Specify the type of burned asset.
- `amount`: `BigInt` - Specify the amount of burned asset.

### Type `SignatureEntry`

A struct for signing on a raw transaction.

Field

- `type_`: `"witness_args_lock"|"witness_args_type"`
- `index`: `Uint32`
- `group_len`: `Uint32`
- `pub_key`: `string` - A key address to figure out private key for signing.
- `sig_type`: `"secp256k1"` - The signature algorithm.

### Type `From`

Fields

- `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid) - Specify the object used to pool asset.
  - If specify an identity, the asset of addresses controlled by the identity will be pooled.
  - If specify an address, the asset of unspent records of the address will be pooled.  
  - If specify the id of an unspent record, the asset of the record will be pooled.
- `source`: `"free"|"claimable"` - Specify the source of asset for paying.

### Type `To`

Fields

- `address`: [`Address`](#type-address) - Specify the receipient's address.
- `mode`: `"HoldByFrom"|"HoldByTo"` - Specify the mode of capacity provided.
- `amount`: `BigInt` - Specify the amount of asset reveived by the receipient.

### Type `SinceConfig`

[Since rule](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0017-tx-valid-since/0017-tx-valid-since.md) is used to prevent a transaction to be spent before a certain block timestamp or a block number.

Fields

- `flag`: `"Relative"|"Absolute"` - Specify the flag of since.
- `type_`: `"BlockNumber"|"EpochNumber"|"Timestamp"` - Specify the type of since.
- `value`: `Uint64` - Specify the value of since.

### Type `SmartTo`

Fields

- `address`: [`Address`](#type-address) - Specify the receipient's address.
- `amount`: `BigInt` - Specify the amount of asset reveived by the receipient.

### Type `ScriptWrapper`

Fields

- `script`: [`Script`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-script)`|null` - Specify the script for querying.
- `io_type`: `"Input"|"Output"|null` - Specify the scope of querying.
  - If set null, it will query in both input and output.
- `args_len`: `Uint32|null` - Specify the length of args in script.

### Type `CellInfo`

Fields

- `cell_output`: [`CellOutput`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-celloutput) - Specify the output of the cell.
- `outpoint`: [`Outpoint`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-outpoint) - Specify the outpoint of the cell.
- `block_hash`: `string` - Specify the hash of block contains the cell.
- `block_number`: `Uint64` - Specify the number of block contains the cell.
- `data`: `string` - Specify the cell data.

### Type `MercuryInfo`

Fields

- `mercury_version`: `string` - Specify the version of mercury.
- `ckb_node_version`: `string` - Specify the version of ckb node.
- `network_type`: `"Mainnet"|"Testnet"|"Staging"|"Dev"` - Specify the type of network.
- `enabled_extensions`: `Array<`[`Extension`](#type-extension)`>` - Specify the extensions enabled.

### Type `Extension`

Fields

- `name`: `string` - Specify the nme of the extension.
- `scripts`: `Array<`[`Script`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-script)`>` - Specify scripts of the extension.
- `cell_deps`: `Array<`[`CellDep`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-celldep)`>` - Specify the cell deps of the extension.

### Type `DBInfo`

Fields

- `version`: `string` - Specify the version of database.
- `db`: `"PostgreSQL"|"MySQL"|"SQLite"` - Specify the version of ckb node.
- `connection_size`: `Uint32` - Specify the connection size of database.
- `center_id`: `Int64` - Specify the center id of database.
- `machine_id`: `Int64` - Specify the machine id of database.
