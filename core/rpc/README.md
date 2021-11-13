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
  - [Method `get_balance`](#method-get_balance)
  - [Method `get_block_info`](#method-get_block_info)
  - [Method `get_transaction_info`](#method-get_transaction_info)
  - [Method `query_transactions`](#method-query_transactions)
  - [Method `build_adjust_account_transaction`](#method-build_adjust_account_transaction)
  - [Method `build_transfer_transaction`](#method-build_transfer_transaction)
  - [Method `build_simple_transfer_transaction`](#method-build_simple_transfer_transaction)
  - [Method `register_addresses`](#method-register_addresses)
  - [Method `build_dao_deposit_transaction`](#method-build_dao_deposit_transaction)
  - [Method `build_dao_withdraw_transaction`](#method-build_dao_withdraw_transaction)
  - [Method `build_dao_claim_transaction`](#method-build_dao_claim_transaction)
  - [Method `get_spent_transaction`](#method-get_spent_transaction)
  - [Method `get_mercury_info`](#method-get_mercury_info)
  - [Method `get_db_info`](#method-get_db_info)
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
  - [Type `SignatureLocation`](#type-signaturelocation)
  - [Type `SignatureAction`](#type-signatureaction)
  - [Type `From`](#type-from)
  - [Type `To`](#type-to)
  - [Type `ToInfo`](#type-toinfo)
  - [Type `SinceConfig`](#type-sinceconfig)
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
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "item": {
        "Address": "ckt1qypyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqjx5d7w"
      },
      "asset_infos": [
        {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000" 
        },
        {
          "asset_type": "UDT",
          "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd" 
        }
      ],
      "tip_block_number": null
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "balances": [{
      "address_or_lock_hash": {
        "Address": "ckt1qypyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqjx5d7w"
      },
      "asset_info": {
        "asset_type": "CKB",
        "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
      },
      "free": "0",
      "occupied": "56800000000",
      "freezed": "0",
      "claimable": "0"
    }, {
      "address_or_lock_hash": {
        "Address": "ckt1qypyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqjx5d7w"
      },
      "asset_info": {
        "asset_type": "UDT",
        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
      },
      "free": "300",
      "occupied": "0",
      "freezed": "0",
      "claimable": "0"
    }],
    "tip_block_number": 3418141
  },
  "id": 42
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_block_info",
  "params": [
    {
      "block_number": 508609,
      "block_hash": null
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "block_number": 508609,
    "block_hash": "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70",
    "parent_hash": "0x1f31dac8331e2041c7d19e57acf078b8a0a4d10531ffa6f59010ed080da9a736",
    "timestamp": 1601357943712,
    "transactions": [{
      "tx_hash": "0x32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af",
      "records": [{
        "id": "32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af0000000000636b743171797164356579796774646d7764723767653733367a77367a306a753677737737727373753866637665",
        "address_or_lock_hash": {
          "Address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve"
        },
        "amount": "161989575784",
        "occupied": 6100000000,
        "asset_info": {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "status": {
          "Fixed": 508609
        },
        "extra": "CellBase",
        "block_number": 508609,
        "epoch_number": 1361210075251467
      }],
      "fee": 0,
      "burn": []
    }]
  },
  "id": 42
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_transaction_info",
  "params": [
    "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3"
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "transaction": {
      "tx_hash": "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3",
      "records": [{
        "id": "26bc4c75669023ca4e599747f9f59184307428ad64c35d00417bd60a95e550a10000000000636b743171797166346e346736716672766e703738727934736d30746e387767706a716636756671373473726c64",
        "address_or_lock_hash": {
          "Address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld"
        },
        "amount": "-14367400000",
        "occupied": 14200000000,
        "asset_info": {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "status": {
          "Fixed": 3418132
        },
        "extra": null,
        "block_number": 3418132,
        "epoch_number": 1979131868744866
      }, {
        "id": "d82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a30000000000636b743171797166346e346736716672766e703738727934736d30746e387767706a716636756671373473726c64",
        "address_or_lock_hash": {
          "Address": "ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld"
        },
        "amount": "14367200000",
        "occupied": 14200000000,
        "asset_info": {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "status": {
          "Fixed": 3418281
        },
        "extra": null,
        "block_number": 3418281,
        "epoch_number": 1979134368550050
      }],
      "fee": 200000,
      "burn": []
    },
    "status": "committed",
    "reject_reason": null
  },
  "id": 42
}
```

### Method `query_transactions`

- `query_transactions(item, asset_infos, extra, block_range, pagination, structure_type)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)
  - `asset_infos`: `Array<`[`AssetInfo>`](#type-assetinfo)`>`
  - `extra`: `"DAO"|"Cellbase" |null`
  - `block_range`: [`Range`](#type-range)`|null`
  - `pagination`: [`PaginationRequest`](#type-paginationrequest)
  - `structure_type`: `"Native"|"DoubleEntry"`
- result
  - `response`: `Array<`[`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithRichStatus`](#type-transactionwithrichstatus)`>`
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_transactions",
  "params": [
    {
      "item": {
        "Address": "ckt1qq95f2qqxn8vj83wyw696sc6krnp6hn420aqa4eew6ky8xrednwkzqd890yus8muntcm04uef0d9heeg0tcxh7ccge0x9"
      },
      "asset_infos": [],
      "extra": null,
      "block_range": null,
      "pagination": {
        "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
        "order": "desc",
        "limit": 50,
        "skip": null,
        "return_count": true
      },
      "structure_type": "DoubleEntry"
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "response": [{
      "TransactionInfo": {
        "tx_hash": "0x305f66236d82316f3d394a796bb16a804ee7ce27751cefd3b842bce5ef0df202",
        "records": [{
          "id": "305f66236d82316f3d394a796bb16a804ee7ce27751cefd3b842bce5ef0df2020000000000636b743171797164356579796774646d7764723767653733367a77367a306a753677737737727373753866637665",
          "address_or_lock_hash": {
            "Address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve"
          },
          "amount": "110948721497",
          "occupied": 6100000000,
          "asset_info": {
            "asset_type": "CKB",
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          },
          "status": {
            "Fixed": 3418348
          },
          "extra": "CellBase",
          "block_number": 3418348,
          "epoch_number": 1979135492623522
        }],
        "fee": 0,
        "burn": []
      }
    }],
    "next_cursor": [0, 52, 40, 236, 0, 0, 0, 1],
    "count": 1
  },
  "id": 42
}
```

### Method `build_adjust_account_transaction`

- `build_adjust_account_transaction(item, from, asset_info, account_number, extra_ckb, fee_rate)`
  - `item`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)
  - `from`: `Array<`[`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)`>`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `account_number`: `Uint32|null`
  - `extra_ckb`: `Uint64|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)`|null`
  - `signature_actions`: `Array<`[`SignatureAction`](#type-signatureaction)`>`

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
  - The elements in the `from` array must be the same kind of enumeration.
- `asset_info` - Specify an asset type for creating asset accounts.
- `account_number` - Specify a target account number.
- `extra_ckb` - Specify the amount of extra CKB injected into an account for paying fees or other usage.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transaction of creating/recycling account.
- `signature_actions` - Signature actions for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_adjust_account_transaction",
  "params": [
    {
      "item": {
        "Identity": "00791359d5f872fc4e72185034da0becb5febce98b"
      },
      "from": [],
      "asset_info": {
        "asset_type": "UDT",
        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
      },
      "account_number": null,
      "extra_ckb": null,
      "fee_rate": null
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [{
        "out_point": {
          "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
          "index": "0x0"
        },
        "dep_type": "dep_group"
      }, {
        "out_point": {
          "tx_hash": "0xec26b0f85ed839ece5f11c4c4e837ec359f5adc4420410f6453b1f6b60fb96a6",
          "index": "0x0"
        },
        "dep_type": "dep_group"
      }, {
        "out_point": {
          "tx_hash": "0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769",
          "index": "0x0"
        },
        "dep_type": "code"
      }],
      "header_deps": [],
      "inputs": [{
        "since": "0x0",
        "previous_output": {
          "tx_hash": "0xfe6760ce7d87418324074e545ac979d695accf0a014ac4eb02f3ab2787ebf2b3",
          "index": "0x2"
        }
      }],
      "outputs": [{
        "capacity": "0x35458af00",
        "lock": {
          "code_hash": "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356",
          "hash_type": "type",
          "args": "0x791359d5f872fc4e72185034da0becb5febce98b"
        },
        "type": {
          "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
          "hash_type": "type",
          "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
        }
      }, {
        "capacity": "0x4b413d7e727a",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x791359d5f872fc4e72185034da0becb5febce98b"
        },
        "type": null
      }],
      "outputs_data": ["0x00000000000000000000000000000000", "0x"],
      "witnesses": ["0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],
      "hash": "0x794b60d2b9a3c2e3c754a5a0bbf891ea7eb7b63ebff3ea9ec46f4f0c2d40105e"
    },
    "signature_actions": [{
      "signature_location": {
        "index": 0,
        "offset": 20
      },
      "signature_info": {
        "algorithm": "Secp256k1",
        "address": "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr"
      },
      "hash_algorithm": "Blake2b",
      "other_indexes_in_group": []
    }]
  },
  "id": 42
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
  - `signature_actions`: `Array<`[`SignatureAction`](#type-signatureaction)`>`

**Usage**

To build a raw transfer transaction and signature actions for signing.

**Params**

- `asset_info` - Specify the asset type for the transfer.
- `from` - Specify the sender.
  - The elements in the `From::items` array must be the same kind of enumeration.

- `to` - Specify recipient's address, amount etc.
- `pay_fee` - Specify the account for paying the fee.
  - If `pay_fee` is null, the `from` address pays the fee.
- `change` - Specify an address for the change.
  - If `change` is null, the first item in `from` works as the change address.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration to prevent the transaction to be spent before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_actions` - Signature actions for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_transfer_transaction",
  "params": [{
    "asset_info": {
      "asset_type": "CKB",
      "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
    },
    "from": {
      "items": [
        {
          "Address": "ckt1qyq90n9s00ngwhmpmymrdv8wzxm82j2xylfq2agzzj"
        }
      ],
      "source": "Free"
    },
    "to": {
      "to_infos": [
        {
          "address": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70",
          "amount": "9650000000"
        }
      ],
      "mode": "HoldByFrom"
    },
    "pay_fee": null,
    "change": null,
    "fee_rate": null,
    "since": {
      "flag": "Absolute",
      "type_": "BlockNumber",
      "value": 6000000
    }
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [
        {
          "out_point": {
            "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
            "index": "0x0"
          },
          "dep_type": "dep_group"
        }
      ],
      "header_deps": [],
      "inputs": [
        {
          "since": "0x5b8d80",
          "previous_output": {
            "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
            "index": "0x9"
          }
        }
      ],
      "outputs": [
        {
          "capacity": "0x23f2f5080",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
          },
          "type": null
        },
        {
          "capacity": "0xba821310c6a38b0",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x57ccb07be6875f61d93636b0ee11b675494627d2"
          },
          "type": null
        }
      ],
      "outputs_data": [
        "0x",
        "0x"
      ],
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ],
      "hash": "0x923da07a578f966209fcd68a418e293dd5c38733fe98b1858a1e89507a238f42"
    },
    "signature_actions": [
      {
        "signature_location": {
          "index": 0,
          "offset": 20
        },
        "signature_info": {
          "algorithm": "Secp256k1",
          "address": "ckt1qyq90n9s00ngwhmpmymrdv8wzxm82j2xylfq2agzzj"
        },
        "hash_algorithm": "Blake2b",
        "other_indexes_in_group": []
      }
    ]
  },
  "id": 42
}
```

### Method `build_simple_transfer_transaction`

- `build_simple_transfer_transaction(asset_info, from, to, change, fee_rate, since)`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `from`: `Array<string>`
  - `to`: [`ToInfo`](#type-toinfo)
  - `change`: `string|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_actions`: `Array<`[`SignatureAction`](#type-signatureaction)`>`

**Usage**

To build a raw transfer transaction and signature actions for signing, and infer `source` and `mode` based on a simple strategy.

**Params**

- `asset_info` - Specify the asset type for the transfer.
- `from` - Specify the senders' addresses. 
- `to` - Specify recipient's address and amount.
- `change` -  Specify an address for the change.
  - If `change` is null, the first address in `from` works as the change address.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration to prevent the transaction to be spent before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_actions` - Signature actions for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_simple_transfer_transaction",
  "params": [{
    "asset_info": {
      "asset_type": "CKB",
      "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
    },
    "from": [
      "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70"
    ],
    "to": [
      {
        "address": "ckt1qyqg88ccqm59ksxp85788pnqg4rkejdgcg2qxcu2qf",
        "amount": "10800000000"
      }
    ],
    "change": null,
    "fee_rate": null,
    "since": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [{
        "out_point": {
          "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
          "index": "0x0"
        },
        "dep_type": "dep_group"
      }],
      "header_deps": [],
      "inputs": [{
        "since": "0x0",
        "previous_output": {
          "tx_hash": "0x7c015aa11672bed4e7bb8756286a57a215cec0b1224d3f05c9233b6799612434",
          "index": "0x1"
        }
      }],
      "outputs": [{
        "capacity": "0x283baec00",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x839f1806e85b40c13d3c73866045476cc9a8c214"
        },
        "type": null
      }, {
        "capacity": "0x6640f2116d113ce",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
        },
        "type": null
      }],
      "outputs_data": ["0x", "0x"],
      "witnesses": ["0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],
      "hash": "0x1d2e5f4eb6a18b313f367a22884cf46ff017bf7798179eaeb6af09ad88f15b2c"
    },
    "signature_actions": [{
      "signature_location": {
        "index": 0,
        "offset": 20
      },
      "signature_info": {
        "algorithm": "Secp256k1",
        "address": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70"
      },
      "hash_algorithm": "Blake2b",
      "other_indexes_in_group": []
    }]
  },
  "id": 42
}
```

### Method `register_addresses`

- `register_addresses(addresses)`
  - `addresses`: `Array<string>`
- result
  A list of lock script hash of the registered addresses.

**Usage**

To reveal the receivers' addresses of a cheque cell. 
Attention: official public servers do not open this method.

**Params**

- `addresses` - Registered addresses.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "register_addresses",
  "params": [
    "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr"
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": [
    "0xca9fc3cbc670e67451e920e6f57c647f529e567f"
  ],
  "id": 42
}
```

### Method `build_dao_deposit_transaction`

- `build_deposit_transaction(from, to, amount, fee_rate)`
  - `from`: [`From`](#type-from)
  - `to`: `string|null`
  - `amount`: `Uint64`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_actions`: `Array<`[`SignatureAction`](#type-signatureaction)`>`

**Usage**

To build a transaction to deposit specified amount of CKB to Dao.

**Params**

- `from` - Specify the provider of the CKB for Dao deposition.
  - The elements in the `From::items` array must be the same kind of enumeration.

- `to` - Specify the recipient of the deposit.
  - If `to` is null, the CKB is deposited to the `from` address.
- `amount` - Specify the amount of CKB for the deposit. The deposit amount should larger than 200 CKB.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_actions` - Signature actions for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_deposit_transaction",
  "params": [{
    "from": {
      "items": [
        {
          "Address": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70"
        }
      ],
      "source": "Free"
    },
    "to": null,
    "amount": 20000000000,
    "fee_rate": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [{
        "out_point": {
          "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
          "index": "0x0"
        },
        "dep_type": "dep_group"
      }, {
        "out_point": {
          "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
          "index": "0x2"
        },
        "dep_type": "code"
      }],
      "header_deps": [],
      "inputs": [{
        "since": "0x0",
        "previous_output": {
          "tx_hash": "0x21a8f6373665fe387b1a781722dcda77e7e926937a236f7a477632d32a7ff144",
          "index": "0x1"
        }
      }],
      "outputs": [{
        "capacity": "0x6640e361dcf259c",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
        },
        "type": null
      }, {
        "capacity": "0x4a817c800",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
        },
        "type": {
          "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e",
          "hash_type": "type",
          "args": "0x"
        }
      }],
      "outputs_data": ["0x", "0x0000000000000000"],
      "witnesses": ["0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],
      "hash": "0x487987835629779f2a03f1919367c163f0851d4eca11d528d85804bc91aaa870"
    },
    "signature_actions": [{
      "signature_location": {
        "index": 0,
        "offset": 20
      },
      "signature_info": {
        "algorithm": "Secp256k1",
        "address": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70"
      },
      "hash_algorithm": "Blake2b",
      "other_indexes_in_group": []
    }]
  },
  "id": 42
}
```

### Method `build_dao_withdraw_transaction`

- `build_dao_withdraw_transaction(from, pay_fee, fee_rate)`
  - `from`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)
  - `pay_fee`: `string|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_actions`: `Array<`[`SignatureAction`](#type-signatureaction)`>`

**Usage**

To build a transaction to withdraw specified deposited CKB from DAO.

**Params**

- `from` - Specify the provider for the deposit cells.
- `pay_fee` - Specify the account for paying the fee.
  - If `pay_fee` is null, the `from` address pays the fee.
- `fee_rate` -  The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_actions` - Signature actions for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_withdraw_transaction",
  "params": [{
    "from": {
      "Address": "ckb1qyqrd0su0thsfgzgts0uvqkmch8f6w85cxrqxgun25"
    },
    "pay_fee": "ckb1qyq8ze8534a9hu3fs9n03kqms84yayywz6ksflfvpk",
    "fee_rate": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-mainnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [{
        "out_point": {
          "tx_hash": "0xe2fb199810d49a4d8beec56718ba2593b665db9d52299a0f9e6e75416d73ff5c",
          "index": "0x2"
        },
        "dep_type": "dep_group"
      }, {
        "out_point": {
          "tx_hash": "0x71a7ba8fc96349fea0ed3a5c47992e3b4084b031a42264a018e0072e8172e46c",
          "index": "0x0"
        },
        "dep_type": "dep_group"
      }],
      "header_deps": ["0xe066ef7fb8b3a888cb19d233940645bf0c125281a4ce67197949cb5d07b2244a"],
      "inputs": [{
        "since": "0x0",
        "previous_output": {
          "tx_hash": "0x2bfb16a742d563f86c2b0b4b3fa5be79a6502a4f5402becdba670be4da56c843",
          "index": "0x0"
        }
      }, {
        "since": "0x0",
        "previous_output": {
          "tx_hash": "0x978868570be4e8eb8e38e3b4464404dd87c6e6c8540ecbd113c0ae33d754371f",
          "index": "0x0"
        }
      }],
      "outputs": [{
        "capacity": "0x212929ada6",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x7164f48d7a5bf2298166f8d81b81ea4e908e16ad"
        },
        "type": null
      }, {
        "capacity": "0x60a24181e4000",
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
          "hash_type": "type",
          "args": "0x36be1c7aef04a0485c1fc602dbc5ce9d38f4c186"
        },
        "type": {
          "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e",
          "hash_type": "type",
          "args": "0x"
        }
      }],
      "outputs_data": ["0x", "0xeb38190000000000"],
      "witnesses": [],
      "hash": "0x00d2c9d74da37b8bc5c15db9c675c94ee6ffb1748e4d092e17e0c964cdbc5801"
    },
    "signature_entries": [{
      "type_": "WitnessLock",
      "index": 0,
      "group_len": 1,
      "pub_key": "ckb1qyq8ze8534a9hu3fs9n03kqms84yayywz6ksflfvpk",
      "signature_type": "Secp256k1"
    }, {
      "type_": "WitnessLock",
      "index": 1,
      "group_len": 1,
      "pub_key": "ckb1qyqrd0su0thsfgzgts0uvqkmch8f6w85cxrqxgun25",
      "signature_type": "Secp256k1"
    }]
  },
  "id": 42
}
```

### Method `build_dao_claim_transaction`

- `build_dao_claim_transaction(from, to, fee_rate)`
  - `from`: [`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)
  - `to`: `string|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `signature_actions`: `Array<`[`SignatureAction`](#type-signatureaction)`>`

**Usage**

To build a transaction to claim specified withdrawing CKB from DAO.

**Params**

- `from` - Specify the provider for the withdrawing cells.
- `to` - Specify the recipient of the claim.
  - If `to` is null, the CKB is claim to the `from` address.
- `fee_rate` -  The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `signature_actions` - Signature actions for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_claim_transaction",
  "params": [{
    "from": {
      "Address": "ckt1qyqzqfj8lmx9h8vvhk62uut8us844v0yh2hsnqvvgc"
    },
    "fee_rate": 1000
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "tx_view": {
    "version": "0x0", 
    "hash": "0x5a504fc1d599e0d946a12f552e71a103390b8649f87f521d30435efb2789a854", 
    "cell_deps": [
      {
        "out_point": {
          "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37", 
          "index": "0x0"
        }, 
        "dep_type": "dep_group"
      }, 
      {
        "out_point": {
          "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f", 
          "index": "0x2"
        }, 
        "dep_type": "code"
      }
    ], 
    "header_deps": [
      "0x2239fe1f3a2f298c7366a56f032917a6094c149db28171bec0feec43eb742097", 
      "0x4abb820e53b9ae4a9e6d4083b7c5f5c67d64ea6b947e397b46ba9cd1a14141b4", 
      "0x983cf4adba409a2d9f3f2887056b57f7304ea7c14d59361a2859e0a8e0ed1ba1", 
      "0x9bf3fb06b5f9181b93865d3a251a7818680cdeada14c0a3a0fdf0070995ede65", 
      "0xf213111b29a2212991f88e8afc4183ffa68e0e7aedc9c1218bc36fb2015974ea", 
      "0x7a52bcc6cc3ba1c303a3aba758511aa8743217d4d31c3b6a26722cbab53426e2", 
      "0x4f980d67afb5eeb6fe072206eb638bccd3f361d84ee33b330121155a603fc88c", 
      "0x8bf26dd69e0cdbb630dd29811da565641ddb4c51cb6643166c98c5e0c2c45ee4", 
      "0x2dd0a8bc184acfd715423c190b4a357c87d5fb5b42a7d0b664f2e7dd42ded7bc", 
      "0xf054532065688d910f1d530fbf0112656c413b7f38247bd6f0739436347b43a8", 
      "0x644925a10c6a55483192ca43e96c91cd787abe4c0a2a560390fccaf45697ebd7", 
      "0x2df41f7aa3a1576076f237682b1bd48ff59cf2342367fbdb57d2829b0c90a28d", 
      "0x4a7c4618b9d8840d9c0e1a57e3433f9310ff2e77a2251bd0f5bcc6f0884fa2de", 
      "0xf8e43b8845ea1fe0210d72041b74bf050c7ad6b9422f4e8fa9ff0df827ad967b", 
      "0x2a64d66794850a5c7ffb3c62017446622699639cf5b5bdf45b4fb4bbf4d53bd2", 
      "0xacae30eeddf20ad0b435e546ee9426a91740b9f52ac321b0fc82e6c39fd989b7", 
      "0x40b070909ec0233ab6b376f4651679ae876a51b46ef0e223396193ab7e140001", 
      "0x4a68eaf75ce8d59801024548cc4de7d9f37e49cd1fa77857ec6279ed169d5f1d"
    ], 
    "inputs": [
      {
        "previous_output": {
          "tx_hash": "0x5b363d68903fe76c17d51d8744fb9c8b33537daf649123d11aa89095d1f8be5d", 
          "index": "0x1"
        }, 
        "since": "0x2000000000000c07"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x7c50ff497761a6e7bade2c2b8ef0a60aa4d50f56d0b9269762b88d8ad574bb6c", 
          "index": "0x1"
        }, 
        "since": "0x2000000000000c02"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x62cfa35c59b43e574585454092d4326b5d4ae3ba9307d3a5e0bb8f1097f99f99", 
          "index": "0x1"
        }, 
        "since": "0x2000000000000c02"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x62cfa35c59b43e574585454092d4326b5d4ae3ba9307d3a5e0bb8f1097f99f99", 
          "index": "0x2"
        }, 
        "since": "0x2000000000000c13"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x62cfa35c59b43e574585454092d4326b5d4ae3ba9307d3a5e0bb8f1097f99f99", 
          "index": "0x3"
        }, 
        "since": "0x2000000000000c13"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x62cfa35c59b43e574585454092d4326b5d4ae3ba9307d3a5e0bb8f1097f99f99", 
          "index": "0x4"
        }, 
        "since": "0x2000000000000c13"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x55ca3f39a3aa2718cc121b7825a98d5a70782932bcbb79ce327dffb6d1df3690", 
          "index": "0x1"
        }, 
        "since": "0x2000000000000c18"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x55ca3f39a3aa2718cc121b7825a98d5a70782932bcbb79ce327dffb6d1df3690", 
          "index": "0x2"
        }, 
        "since": "0x2000000000000c18"
      }, 
      {
        "previous_output": {
          "tx_hash": "0x55ca3f39a3aa2718cc121b7825a98d5a70782932bcbb79ce327dffb6d1df3690", 
          "index": "0x3"
        }, 
        "since": "0x2000000000000c18"
      }, 
      {
        "previous_output": {
          "tx_hash": "0xd7070174ded5a5db614f4744bed909810469162fc589e0434874530cb4cef19c", 
          "index": "0x1"
        }, 
        "since": "0x2000000000000c37"
      }, 
      {
        "previous_output": {
          "tx_hash": "0xd7070174ded5a5db614f4744bed909810469162fc589e0434874530cb4cef19c", 
          "index": "0x2"
        }, 
        "since": "0x2000000000000c38"
      }, 
      {
        "previous_output": {
          "tx_hash": "0xd7070174ded5a5db614f4744bed909810469162fc589e0434874530cb4cef19c", 
          "index": "0x3"
        }, 
        "since": "0x2000000000000c38"
      }, 
      {
        "previous_output": {
          "tx_hash": "0xd7070174ded5a5db614f4744bed909810469162fc589e0434874530cb4cef19c", 
          "index": "0x4"
        }, 
        "since": "0x2000000000000c39"
      }
    ], 
    "outputs": [
      {
        "capacity": "0x5ad12abb85", 
        "lock": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "args": "0x202647fecc5b9d8cbdb4ae7167e40f5ab1e4baaf", 
          "hash_type": "type"
        }
      }
    ], 
    "outputs_data": [
      "0x"
    ], 
    "witnesses": [
      "0x61000000100000005500000061000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000", 
      "0x1c00000010000000100000001c000000080000000200000000000000", 
      "0x1c00000010000000100000001c000000080000000400000000000000", 
      "0x1c00000010000000100000001c000000080000000600000000000000", 
      "0x1c00000010000000100000001c000000080000000700000000000000", 
      "0x1c00000010000000100000001c000000080000000800000000000000", 
      "0x1c00000010000000100000001c000000080000000900000000000000", 
      "0x1c00000010000000100000001c000000080000000b00000000000000", 
      "0x1c00000010000000100000001c000000080000000c00000000000000", 
      "0x1c00000010000000100000001c000000080000000d00000000000000", 
      "0x1c00000010000000100000001c000000080000000f00000000000000", 
      "0x1c00000010000000100000001c000000080000001000000000000000", 
      "0x1c00000010000000100000001c000000080000001100000000000000"
    ]
  }, 
  "signature_actions": [
    {
      "signature_location": {
        "index": 0, 
        "offset": 20
      }, 
      "signature_info": {
        "algorithm": "Secp256k1", 
        "address": "ckt1qyqzqfj8lmx9h8vvhk62uut8us844v0yh2hsnqvvgc"
      }, 
      "hash_algorithm": "Blake2b", 
      "other_indexes_in_group": [1,2,3,4,5,6,7,8,9,10,11,12]
    }
  ]
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_spent_transaction",
  "params": [{
    "outpoint": {
      "tx_hash": "0xb2e952a30656b68044e1d5eed69f1967347248967785449260e3942443cbeece",
      "index": "0x1"
    },
    "structure_type": "DoubleEntry"
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "TransactionInfo": {
      "tx_hash": "0x2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed",
      "records": [{
        "id": "b2e952a30656b68044e1d5eed69f1967347248967785449260e3942443cbeece0000000100636b74317179716738386363716d35396b7378703835373838706e716734726b656a646763673271786375327166",
        "address_or_lock_hash": {
          "Address": "ckt1qyqg88ccqm59ksxp85788pnqg4rkejdgcg2qxcu2qf"
        },
        "amount": "-934896986500",
        "occupied": 6100000000,
        "asset_info": {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "status": {
          "Fixed": 2652086
        },
        "extra": null,
        "block_number": 2652086,
        "epoch_number": 1979141314317046
      }, {
        "id": "2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed0000000000636b74317179716738386363716d35396b7378703835373838706e716734726b656a646763673271786375327166",
        "address_or_lock_hash": {
          "Address": "ckt1qyqg88ccqm59ksxp85788pnqg4rkejdgcg2qxcu2qf"
        },
        "amount": "10000000000",
        "occupied": 6100000000,
        "asset_info": {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "status": {
          "Fixed": 2713193
        },
        "extra": null,
        "block_number": 2713193,
        "epoch_number": 1979139754035992
      }, {
        "id": "2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed0000000100636b7431717971306a74797033766d767064353336687735713836647964736b68656561357330716c7364303332",
        "address_or_lock_hash": {
          "Address": "ckt1qyq0jtyp3vmvpd536hw5q86dydskheea5s0qlsd032"
        },
        "amount": "924896985999",
        "occupied": 6100000000,
        "asset_info": {
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "status": {
          "Fixed": 2713193
        },
        "extra": null,
        "block_number": 2713193,
        "epoch_number": 1979139754035992
      }],
      "fee": 501,
      "burn": []
    }
  },
  "id": 42
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_mercury_info",
  "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "mercury_version": "0.2.0-beta.4",
    "ckb_node_version": "v0.101",
    "network_type": "Testnet",
    "enabled_extensions": []
  },
  "id": 42
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

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_db_info",
  "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

- Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "version": "0.2.0-beta.2",
    "db": "PostgreSQL",
    "conn_size": 1000,
    "center_id": 0,
    "machine_id": 0
  },
  "id": 42
}
```

## RPC Types

### Type `Identity`

Fields

- `identity` (Type: `string`): Specify an identity.

### Type `Address`

Fields

- `address` (Type: `string`): Specify an address.

### Type `RecordId`

Fields

- `record_id` (Type: `string`): Specify the ID of a record.

### Type `AssetInfo`

Fields

- `asset_type` (Type: `"CKB"`|`"UDT"`): Specify the asset type.
- `udt_hash` (Type: `string`): Specify the hash of a UDT asset.

### Type `Balance`

Fields

- `address` (Type: `string`): Specify the address that the balance belongs to.
- `asset_info` (Type: [`AssetInfo`](#type-assetinfo): Specify the asset type of the balance.
- `free` (Type: `string`): Specify the amount of freely spendable assets.
- `occupied` (Type: `string`): Specify the amount of CKB that provides capacity.
- `freezed` (Type: `string`): Specify the amount of locked assets.
- `claimable` (Type: `string`): Specify the amount of UDT assets on the cheque cell that are unclaimed and not timed out.

### Type `Range`

Fields

- `from` (Type: `Uint64`): Specify the start block number of the range.
- `to  ` (Type: `Uint64`): Specify the end block number of the range.

### Type `PaginationRequest`

This is a cursor-based pagination configuration.

Fields

- `cursor` (Type:`Array<Uint8>` ): Specify the beginning cursor for the query.
  - If `cursor` is `[127, 255, 255, 255, 255, 255, 255, 254]`, the query starts from the biggest cursor for descending order and from the smallest cursor for ascending order.
- `order  ` (Type: `"Asc"`|`"Desc"`): Specify the order of the returning data.
- `limit` (Type: `Uint64`|`null` ): Specify the entry limit per page of the query.
  - If `limit` is null, a default limit such as 50 will be used.
- `total_count` (Type: `bool`): Specify whether to return the total count.

### Type `BlockInfo`

A double-entry style blockchain structure.

Fields

- `block_number` (Type: `Uint64`): Specify the block number.
- `block_hash` (Type: `string`): Specify the block hash.
- `parent_block_hash` (Type: `string`): Specify the parent block hash.
- `timestamp` (Type: `Uint64`): Specify the timestamp.
- `transactions` (Type:  `Array<`[`TransactionInfo`](#type-transactioninfo)`>`): Specify double-entry style transactions in the block.

### Type `TransactionInfo`

A double-entry style transaction structure.

Fields

- `tx_hash` (Type: `string`): Specify the transaction hash.
- `records`  (Type: `Array<`[`Record`](#type-record)`>`): Specify the records in the transaction.
- `fee` (Type: `Uint64`):  Specify the fee for the transaction.
- `burn` (Type: `Array<`[`BurnInfo`](#type-burninfo)`>`): Specify the amount of burned UDT assets in the transaction.
- `timestamp` (Type: `Uint64`): Specify the timestamp of the block in which the transaction is packaged.

### Type `TransactionWithRichStatus`

A native style transaction structure.

Fields

- `transaction` (Type: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview) `|` `null` ): Specify the transaction.
- `tx_status` (Type: [`TxRichStatus`](#type-txrichstatus)): Specify the transaction status.

### Type `TxRichStatus`

Transaction rich status.

Fields

- `status` (Type: [`Status`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-status)): The transaction status, allowed values: "pending", "proposed" "committed" "unknown" and "rejected".
- `block_hash` (Type: [`H256`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-h256) `|` `null`): Specify the block hash of the block which has committed this transaction in the canonical chain.
- `reason` (Type: `string` `|` `null`): Specify the reason why the transaction is rejected.
- `timestamp` (Type: `Uint64` `|` `null`): Specify the timestamp of the block in which the transaction is packaged.

### Type `Record`

A double-entry style structure that is designed to reflect the asset amount changes of an address in a transaction.

Fields

- `id` (Type: `string`): Specify the identify of the record.
- `address` (Type: `string`): Specify the address of which amounts changed.
- `amount` (Type: `BigInt`): Specify the amount changes.
  - The value is negative when the record is spent, and positive when the record is new.
- `asset_type` (Type: [`AssetInfo`](#type-assetinfo)): Specify the asset type of the record.
- `status` (Type: [`Claimable`](#type-claimable)`|`[`Fixed`](#type-fixed)):  Specify the status of the record.
- `extra` (Type:  [`DaoInfo`](#type-daoinfo)`|"Cellbase"|null`): Specify extra information of the record.
- `epoch_number` (Type: `Uint64`): Epoch value encoded.

### Type `Claimable`

Fields

- `block_number` (Type: `Uint64`): Specify the block number of the block that contains a cheque creation transaction.

### Type `Fixed`

Fields

- `block_number` (Type: `Uint64`): Specify the block number of the block that contains a transaction fixed record.

### Type `DaoInfo`

Fields

- `state`  (Type:[`Deposit`](#type-deposit)`|`[`Withdraw`](#type-withdraw)): Specify the state of a DAO operation.
- `reward` (Type: `Uint64`): Specify the accumulate reward of a DAO operation.

### Type `Deposit`

Fields

- `Deposit` (Type: `Uint64`): Specify the block number of the block that contains a DAO deposit transaction.

### Type `Withdraw`

Fields

- `Withdraw` (Type: `Array<Uint64>`): Specify two block numbers, first block contains a DAO deposit transaction and last block contains the corresponding DAO withdraw transaction.

### Type `BurnInfo`

Fields

- `udt_hash` (Type: `string`):  Specify the type of burned assets.
- `amount` (Type: `string`):  Specify the amount of burned asset.

### Type `SignatureLocation`

Fields

- index(Type: `usize`): Specify the index in witensses vector.
- offset(Type: `usize`): Specify the start byte offset in witness encoded bytes.

### Type `SignatureAction`

A struct for signing on a raw transaction.

Field

- `signature_location` (Type: [`SignatureLocation`](#type-signatureaction)): Specify the location of the signature in the witnesses.
- `signature_info` (Type: `"Secp256k1"`): Specify the signature algorithm and related parameters.
- `hash_algorithm` (Type: `"Blake2b"`): Specify hash algorithm.
- `other_indexes_in_group` (Type: `Vec<usize>`): Indexes of other inputs in the same lock group.

### Type `From`

Fields

- `items`  (Type: `Array<`[`Identity`](#type-identity)`|`[`Address`](#type-address)`|`[`RecordId`](#type-recordid)`>`): Specify the object that pools the assets.
  - If `item` is an identity, the assets of addresses controlled by the identity will be pooled.
  - If `item` is an address, the assets of unspent records of the address will be pooled.
  - If `item` is the ID of an unspent record, the assets of the record will be pooled.
- `source`  (Type: `"free"|"claimable"`): Specify the asset source for the payment.

### Type `To`

Fields

- `to_infos`(Type: `Array<`[`ToInfo`](#type-toinfo)`>`): Specify the recipient's address and transfer amount.
- `mode`  (Type:`"HoldByFrom"|"HoldByTo"`): Specify the mode of the provided capacity.

### Type `ToInfo`

Fields

- `address` (Type: `string`): Specify the recipient's address.
- `amount`  (Type: `string`): Specify the amount of the asset received by the recipient.

### Type `SinceConfig`

The [since rule](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0017-tx-valid-since/0017-tx-valid-since.md)  is used to prevent a transaction to be spent before a certain block timestamp or a block number

Fields

- `flag` (Type: `"Relative"|"Absolute"`): Specify the flag of since.
- `type_` (Type: `"BlockNumber"|"EpochNumber"|"Timestamp"`): Specify the type of since.
- `value` (Type: `Uint64` ): Specify the value of since.

### Type `MercuryInfo`

Fields

- `mercury_version` (Type: `string`): Specify the version of mercury.
- `ckb_node_version` (Type: `string`): Specify the version of a CKB node.
- `network_type` (Type: `"Mainnet"|"Testnet"|"Staging"|"Dev"`): Specify the network type.
- `enabled_extensions` (Type: `Array<`[`Extension`](#type-extension)`>`): Specify the enabled extensions.

### Type `Extension`

Fields

- `name` (Type: `string`): Specify the extension name.
- `scripts` (Type: `Array<`[`Script`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-script)`>`): Specify scripts of the extension.
- `cell_deps` (Type: `Array<`[`CellDep`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-celldep)`>`): Specify the cell dependencies of the extension.

### Type `DBInfo`

Fields

- `version` (Type: `string`): Specify the version of database.
- `db` (Type: `"PostgreSQL"|"MySQL"|"SQLite"`): Specify the version of the CKB node.
- `connection_size` (Type: `Uint32`): Specify the connection size of the database.
- `center_id` (Type: `Int64`): Specify the center ID of the database.
- `machine_id` (Type: `Int64`): Specify the machine ID of the database.
