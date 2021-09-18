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
- Align the design for balance type with CKB.
- Make the core terms more accurate.
- Optimize the design of original interfaces.
- Support DAO query, deposit and withdraw operations.
- Support to get a spent transaction according to an outpoint.
- Add an interface for adjusting the number of ACP cells (creating and destroying cells are automatically inferred).
- Provide more friendly error code design for applications.

## Core Concept

Before exploring the Mercury interfaces, it is crucial to understand Mercury's core concepts.

### Identity

CKB [Cell Model](https://docs.nervos.org/docs/basics/concepts/cell-model) is similar to that of [UTXO](https://en.wikipedia.org/wiki/Unspent_transaction_output) in Bitcoin's terminology. Cell is the basic unit in CKB. The full set of unspent cells in CKB is considered being the full state of CKB at that particular point in time. A [lock script](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#42-cell) defines the ownership of a cell.

Identity is an abstract concept that is used manage lock scripts of the same ownership. For example, an secp256k1 private key can unlock many lock scripts including [secp256k1/blake160](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#42-cell), [acp](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0026-anyone-can-pay/0026-anyone-can-pay.md) and [cheque](https://talk.nervos.org/t/sudt-cheque-deposit-design-and-implementation/5209). The secp256k1 public key corresponding to this private key can act as the identity of these lock scripts.

The structure of identity is `<1 byte flag> <20 bytes identity content>`. Identity is also used in [RCE](https://talk.nervos.org/t/rfc-regulation-compliance-lock/5788) and [pw-core](https://github.com/lay2dev/pw-core).

- `flag`: The flexible nature of CKB lock script design enables CKB to support any public chains' signature algorithm. For example, the community's pw-core project supports both the signature algorithms of BTC, ETH, EOS, TRON, and Doge etc. `flag` is used to distinguish the public keys of these different algorithms. Identity can also support complex contracts that cannot use a single public key to identify the ownership.

- `content`: If a flag represents a public key, then `content` is the blake160 hash of the public key. Otherwise, `content` is the blake160 hash of the lock.

### Address

Mercury supports [ckb address format](https://github.com/nervosnetwork/rfcs/tree/master/rfcs/0021-ckb-address-format) that is encoded from lock scripts. In the near future, Mercury will also support address specifications of some other public chains like BTC, ETH, EOS, TRON, and Doge etc. The support for other addresses will be implemented when Mercury supports pw-core.

### Mode

The Common Knowledge Byte (CKByte, the abbreviation is CKB) is the native token of the Nervos Common Knowledge Base. Custom token standards such as [sUDT](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0025-simple-udt/0025-simple-udt.md) (simple User-Defined Token) and [xUDT](https://talk.nervos.org/t/rfc-extensible-udt/5337) (Extensible User-Defined Token) are also supported. Anyone can create and issue custom tokens on CKB based on these standards.

CKB solves the problem of [state explosion](https://medium.com/@happypeter1983/what-is-blockchain-state-explosion-22dd531eeb21) by using a unique [economic model](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#5-economic-model) design. In this design, CKByte plays the roles of assets and also provides capacity. However, UDT assets cannot provides capacity and they only exist along with a certain amount of CKBytes.

Mode is used to specify whether the sender or the recipient provides the CKBytes to provides capacity for the output cell in a transfer.

- HoldByFrom: The sender provides CKBytes for the output cell.

- HoldByTo: The recipient provides CKBytes for the output cell.

### Balance Type

- free: The assets are freely spendable.
- occupied: The CKBytes are used to provide capacity.
- freezed: The assets such as cellbase, dao etc. are locked, and have not been unlocked.
- claimbale: The UDT assets on the cheque cell are unclaimed and not timed out.

### Source

Only free and claimable balance is spendable and can be used as source in a transfer.

### Double-entry Style Structure

Mercury has a double-entry style blockchain data structure ([`BlockIfo`](#type-blockinfo) -> [`TransactionInfo`](#type-transactioninfo) -> [`Record`](#type-record)) that is abstracted on top of the CKB data structure. The `Record` type is designed to reflect the asset amount changes of an address in a transaction.

### Error Code

Mercury, as the middle layer to the application layer, must provide fixed error codes that applications can handle conveniently.
The error code ranges are as follows:
- The general error code is -1 ~ -999.
- The error code range of the ckb-rpc interface is -1000 ~ -2999.
- The error code range of Mercury is -10000 ~ -12999
- RPC error codes is pre-defined [here](https://github.com/nervosnetwork/ckb/blob/cef2a32d31db8cfe73c634f7f1c52b86c4a8f404/rpc/src/error.rs#L15).

## RPC Methods

### Method `get_balance`

- `get_balance(item, asset_infos, tip_block_number)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `asset_infos`: `Array<`[`AssetInfo>`](#type-assetinfo)`>`
  - `tip_block_number`: `Uint64|null`
- result
  - `tip_block_number`: `Uint64`
  - `balances`: `Array<`[`Balance`](#type-balance)`>`

**Usage**

To return the balance of specified assets for the given item.

**Params**

- `item` - Specify the object for getting the balance.
  - If `item` is an identity, the balance of the addresses controlled by the identity will be accumulated.
  - If `item` is an address, the balance of the unspent records of the address will be accumulated.
  - If `item`  is the ID of an unspent record, the balance of the record will be returned.
- `tip_block_number` - Specify a block of giving block_number as the tip of the blockchain for the query.
  - If `tip_block_number` is null, the query is based on the latest blockchain.
  - If `tip_block_number` is not null, the query is based on the historical blockchain with the specified tip.
- `asset_infos` - Specify a set of asset types for the query.
  - If `asset_infos` is empty, the query returns the balance of any asset matching the query parameters.

**Returns**

- `tip_block_number` - Show the tip of the blockchain for the query.
- `balances` - Show a list of balance information matching the query.

**Examples**

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

**Usage**

To return the double-entry style block structure of a specified block.

**Params**

- `block_number` - Specify the block number for the query.
- `block_hash` - Specify the block hash for the query.

**Returns**

- If both `block_number` and `block_hash` are `null`, the query returns the latest block.
- If `block_number` is `null` and `block_hash` is not `null`, the query returns the block matches `block_hash`.
- If `block_number` is not `null` and `block_hash` is `null`, the query returns the block on the canonical chain matches `block_number`.
- If both `block_number` and `block_hash` are not `null`, the query returns the block on the canonical chain both matching `block_number` and `block_hash`.

**Examples**

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

**Usage**

To return the double-entry style transaction along with the status of a specified transaction hash.

**Params**

- `tx_hash` - Specify the transaction hash for the query.

**Returns**

- `transaction` - double-entry style transaction of the specified `tx_hash`.
- `status`
  - Status "pending" means the transaction is in the pool and not proposed yet.
  - Status "proposed" means the transaction is in the pool and has been proposed.
  - Status "committed" means the transaction has been committed to the canonical chain.
  - Status "Rejected" means the transaction has been rejected by the pool.
  - Status "Unknown" means the transaction was unknown for the pool.
- `reject_reason` - If the transaction is "Rejected",  the query returns the code of the rejection reason.

**Examples**

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

- `query_transactions(item, asset_infos, extra, block_range, pagination, structure_type)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `asset_infos`: `Array<`[`AssetInfo>`](#type-assetinfo)`>`
  - `extra`: `"DAO"|"Cellbase" |null`
  - `block_range`: [`Range`](#type-range)`|null`
  - `pagination`: [`PaginationRequest`](#type-paginationrequest)
  - `structure_type`: `"Native"|"DoubleEntry"`
- result
  - `response`: `Array<`[`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithStatus`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionwithstatus)`>`
  - `next_cursor`: `string|null`
  - `total_count`: `Uint64|null`

**Usage**

To return generic transactions and pagination settings from practical searching.

**Params**

- `item` - Specify the object used to query the involved transactions.
  - If `item` is an identity, the query returns the transactions that involve addresses controlled by the identity.
  - If `item` is an address, the query returns the transactions that involve records of the address.
  - If `item` is the ID of a record, the query returns the transactions that involve the record.
- `asset_infos` - Specify a set of asset types for the query.
  - If `asset_infos` is empty, the query returns the transactions that involve any asset matching the query.
- `extra` - Specify the filter applying to the querying.
  - If `extra` is null, the query does not apply extra filter.
- `block_range` - Specify the block range for the query.
- `pagination` - Specify the pagination set.
  - If `pagination` is null, no pagination set will be applied.
- `structure_type` - Specify the structure type of the transactions.
  - If `structure_type` is Native, the query returns CKB native structure of the transactions.
  - If `structure_type` is DoubleEntry, the query returns the double-entry style structure of the transactions.

**Returns**

- `response` - Return a list of transactions meets the query.
- `next_cursor` - Return the beginning cursor for the next query.
  - If `next_cursor` is null, there's no further transactions matching the query.
- `total_count` - The total count of transactions matching the query and ignoring pagination set. `total_count` can be used for calculating total pages.

**Examples**

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

- `build_adjust_account_transaction(item, from, asset_info, account_number, extra_ckb, fee_rate)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)
  - `from`: `Array<`[`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordID`](#type-recordid)`>`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `account_number`: `Uint32|null`
  - `extra_ckb`: `Uint64|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)`|null`
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

**Usage**

To control the quantity of accounts.

In CKB, users must create asset accounts for receiving UDT assets. Each account occupies 142 CKB and is bound with one single asset type. Normally, it's enough for a user to create one account for one asset. But sometimes, business users need multiple accounts to receive multiple transfers at the same time.

- If the number of existing accounts is below the required number, `build_adjust_account_transaction` builds transactions to create the accounts.
- If the number of existing accounts is above the required number, `build_adjust_account_transaction` builds transactions to recycle the accounts. The UDT assets in a recycled account will move to the remained account.
  - If the amount of the UDT assets is not zero, the required number cannot be zero. At least one account is required to store the UDT assets.
- If the number of existing accounts is equal to the required number, `build_adjust_account_transaction` does not build any transaction.

**Params**

- `item` - Specify the object for creating or recycling accounts.
  - If `item` is an identity, the account controlled by the identity will be created or recycled.
  - If `item` is an address, the account controlled by the identity that is behind the address will be created or recycled.
  - If `item` is the ID of a record, the account controlled by the identity that is behind the record will be created or recycled.
- `from` - Specify the object for providing CKB for creating asset accounts.
  - If `from` is null, the method obtains CKB from `item`.
- `asset_info` - Specify an asset type for creating asset accounts.
- `account_number` - Specify a target account number.
- `extra_ckb` - Specify the amount of extra CKB injected into an account for paying fees or other usage.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transaction of creating/recycling account.
- `signature_entries` - Signature entries for signing.

**Examples**

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

- `build_transfer_transaction(asset_info, from, to, change, fee_rate, since)`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `from`: [`From`](#type-from)
  - `to`: [`To`](#type-to)
  - `pay_fee`: `string|null`
  - `change`: `string|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

**Usage**

To build a raw transfer transaction and signature entries for signing.

**Params**

- `asset_info` - Specify the asset type for the transfer.
- `from` - Specify the sender.
- `to` - Specify recipient's address, amount etc.
- `pay_fee` - Specify the account for paying the fee.
  - If `pay_fee` is null, the `from` address pays the fee.
- `change` - Specify an address for the change.
  - If `change` is null, the first item in `from` works as the change address.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration to prevent the transaction to be spent before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

**Examples**

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

- `build_smart_transfer_transaction(asset_info, from, to, change, fee_rate, since)`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `from`: `Array<string>`
  - `to`: [`ToInfo`](#type-toinfo)
  - `change`: `string|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

**Usage**

To build a raw transfer transaction and signature entries for signing, and infer `source` and `mode` based on a smart strategy.

**Params**

- `asset_info` - Specify the asset type for the transfer.
- `from` - Specify the sender.
- `to` - Specify recipient's address and amount.
- `change` -  Specify an address for the change.
  - If `change` is null, the first address in `from` works as the change address.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration to prevent the transaction to be spent before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

**Examples**

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
  - `addresses`: `Array<string>`
- result
  A list of lock script hash of the registered addresses.

**Usage**

To reveal the receivers' addresses of a cheque cell.

**Params**

- `addresses` - Registered addresses.

**Examples**

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
  - `from`: [`From`](#type-from)
  - `to`: `string|null`
  - `amount`: `Uint64`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

**Usage**

To build a transaction to deposit specified amount of CKB to Dao.

**Params**

- `from` - Specify the provider of the CKB for Dao deposition.
- `to` - Specify the recipient of the deposit.
  - If `to` is null, the CKB is deposited to the `from` address.
- `amount` - Specify the amount of CKB for the deposit.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

**Examples**

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
  - `pay_fee`: `string|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_entries`: `Array<`[`SignatureEntry`](#type-signatureentry)`>`

**Usage**

To build a transaction to withdraw specified deposited CKB from DAO.

**Params**

- `from` - Specify the provider for the deposit cells.
- `pay_fee` - Specify the account for paying the fee.
  - If `pay_fee` is null, the `from` address pays the fee.
- `fee_rate` -  The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_entries` - Signature entries for signing.

**Examples**

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
  - `structure_type`: `"Native"|"DoubleEntry"`
- result
  - `transaction`: [`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithStatus`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionwithstatus)

**Usage**

To obtain the transaction that uses the specified outpoint as the input.

**Params**

- `outpoint` - Specify the outpoint for the query.
- `structure_type` - Specify the structure type of the returning transaction.

**Returns**

- `transaction` - The spent transaction.

**Examples**

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

### Method `get_mercury_info`

- `get_mercury_info()`
- result
  - `mercury_info`: [`MercuryInfo`](#type-mercuryinfo)

**Usage**

To get the information of Mercury.

**Returns**

- `mercury_info` - The information of Mercury.

**Examples**

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

**Usage**

To get the information of the database.

**Returns**

- `db_info` - The information of the database.

**Examples**

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

- `Identity`: `string` - Specify an identity.

### Type `Address`

Fields

- `Address`: `string` - Specify an address.

### Type `RecordId`

Fields

- `Record`: `string` - Specify the id of a record.

### Type `AssetInfo`

Fields

- `asset_info`: `"CKB"|"UDT"` - Specify the type of an asset.
- `udt_hash`: `string` - Specify the hash of an udt asset.

### Type `Balance`

Fields

- `address`: `string` - Specify the address of the balance belongs to.
- `asset_info`: [`AssetInfo`](#type-assetinfo) - Specify the asset type of the balance.
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
- `asset_info`: [`AssetInfo`](#type-assetinfo) - Specify the asset type of the record.
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
- `signature_type`: `"secp256k1"` - The signature algorithm.

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
