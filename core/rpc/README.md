# Mercury JSON-RPC Protocols

- [Major Changes Compared to Version 0.1.0](#major-changes-compared-to-version-010)
- [Core Concept](#core-concept)
  - [Identity](#identity)
  - [Address](#address)
  - [Balance Type](#balance-type)
  - [Double-entry Style Structure](#double-entry-style-structure)
  - [Error Code](#error-code)
- [RPC Methods](#rpc-methods)
  - [Method `get_balance`](#method-get_balance)
  - [Method `get_block_info`](#method-get_block_info)
  - [Method `get_transaction_info`](#method-get_transaction_info)
  - [Method `query_transactions`](#method-query_transactions)
  - [Method `get_account_info`](#method-get_account_info)
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
  - [Method `build_sudt_issue_transaction`](#method-build_sudt_issue_transaction)
  - [Method `get_sync_state`](#method-get_sync_state)
  - [Method `start_profiler`](#method-start_profiler)
  - [Method `report_pprof`](#method-report_pprof)
- [RPC Types](#rpc-types)
  - [Type `JsonItem`](#type-jsonitem)
  - [Type `AssetInfo`](#type-assetinfo)
  - [Type `Balance`](#type-balance)
  - [Type `Range`](#type-range)
  - [Type `PaginationRequest`](#type-paginationrequest)
  - [Type `BlockInfo`](#type-blockinfo)
  - [Type `TxView`](#type-txview)
  - [Type `TransactionInfo`](#type-transactioninfo)
  - [Type `TransactionWithRichStatus`](#type-transactionwithrichstatus)
  - [Type `TxRichStatus`](#type-txrichstatus)
  - [Type `Record`](#type-record)
  - [Type `ExtraFilter`](#type-extrafilter)
  - [Type `DaoInfo`](#type-daoinfo)
  - [Type `DaoState`](#type-daoState)
  - [Type `BurnInfo`](#type-burninfo)
  - [Type `ScriptGroup`](#type-scriptgroup)
  - [Type `ToInfo`](#type-toinfo)
  - [Type `SinceConfig`](#type-sinceconfig)
  - [Type `MercuryInfo`](#type-mercuryinfo)
  - [Type `Extension`](#type-extension)
  - [Type `DBInfo`](#type-dbinfo)
  - [Type `SyncState`](#type-syncstate)
  - [Type `SyncProgress`](#type-syncprogress)
  - [Type `Uint32`](#type-uint32)
  - [Type `Uint64`](#type-uint64)
  - [Type `Uint128`](#type-uint128)
  - [Type `BlockNumber`](#type-blocknumber)


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

### Balance Type

- free: unlimited spendable balance.
- occupied: unspendable balance which is occupied by offering capacity. Only CKByte has this category.
- frozen: unspendable balance besides occupied.

### Double-entry Style Structure

Mercury has a double-entry style blockchain data structure ([`BlockInfo`](#type-blockinfo) -> [`TransactionInfo`](#type-transactioninfo) -> [`Record`](#type-record)) that is abstracted on top of the CKB data structure. The `Record` type is designed to reflect the asset amount changes of an address in a transaction.

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
  - `item`: [`JsonItem`](#type-jsonitem)
  - `asset_infos`: `Array<`[`AssetInfo`](#type-assetinfo)`>`
  - `tip_block_number`: [`BlockNumber`](#type-blocknumber)`|null`
- result
  - `tip_block_number`: [`BlockNumber`](#type-blocknumber)
  - `balances`: `Array<`[`Balance`](#type-balance)`>`

**Usage**

To return the balance of specified assets for the given item.

**Params**

- `item` - Specify the object for getting the balance.
  - If `item` is an identity, the balance of the addresses controlled by the identity will be accumulated.
  - If `item` is an address, the balance of the unspent records of the address will be accumulated.
  - If `item`  is an unspent out point, the balance of the record will be returned.
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
        "type": "Address", 
        "value": "ckt1qypyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqjx5d7w"
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
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "balances": [
      {
        "ownership": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq2yjd0tepsduzpg9vqkx5mkjlndd5vdr7svk06gk", 
        "asset_info": {
          "asset_type": "CKB", 
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        }, 
        "free": "0x0", 
        "occupied": "0xd398b3800", 
        "frozen": "0x0"
      }, 
      {
        "ownership": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq2yjd0tepsduzpg9vqkx5mkjlndd5vdr7svk06gk", 
        "asset_info": {
          "asset_type": "UDT", 
          "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
        }, 
        "free": "0x12c", 
        "occupied": "0x0", 
        "frozen": "0x0"
      }
    ], 
    "tip_block_number": "0x4eabe1"
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
      "block_number": "0x7c2c1",
      "block_hash": null
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "block_number": "0x7c2c1", 
    "block_hash": "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70", 
    "parent_hash": "0x1f31dac8331e2041c7d19e57acf078b8a0a4d10531ffa6f59010ed080da9a736", 
    "timestamp": "0x174d85f13a0", 
    "transactions": [
      {
        "tx_hash": "0x32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af", 
        "records": [
          {
            "out_point": {
              "tx_hash": "0x32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af", 
              "index": "0x0"
            }, 
            "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqw6vjzy9kahx3lyvlgap8dp8ewd8g80pcgcexzrj", 
            "io_type": "Output", 
            "amount": "0x25b754c468", 
            "occupied": "0x0", 
            "asset_info": {
              "asset_type": "CKB", 
              "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }, 
            "extra": {
              "type": "Cellbase"
            }, 
            "block_number": "0x7c2c1", 
            "epoch_number": "0x4d6036b00030b"
          }
        ], 
        "fee": "0x0", 
        "burn": [ ], 
        "timestamp": "0x174d85f13a0"
      }
    ]
  }, 
  "id": 42
}
```

### Method `get_transaction_info`

- `get_transaction_info(tx_hash)`
  - `tx_hash`: `string`
- result
  - `transaction`: [`TransactionInfo`](#type-transactioninfo)`|null`
  - `status`: `"Pending"|"Proposed"|"Committed"|"Rejected"|"Unknown"`

**Usage**

To return the double-entry style transaction along with the status of a specified transaction hash.

**Params**

- `tx_hash` - Specify the transaction hash for the query.

**Returns**

- `transaction` - double-entry style transaction of the specified `tx_hash`.
- `status`
  - Status "Pending" means the transaction is in the pool and not proposed yet.
  - Status "Proposed" means the transaction is in the pool and has been proposed.
  - Status "Committed" means the transaction has been committed to the canonical chain.
  - Status "Rejected" means the transaction has been rejected by the pool.
  - Status "Unknown" means the transaction was unknown for the pool.

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
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "transaction": {
      "tx_hash": "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3", 
      "records": [
        {
          "out_point": {
            "tx_hash": "0x26bc4c75669023ca4e599747f9f59184307428ad64c35d00417bd60a95e550a1", 
            "index": "0x0"
          }, 
          "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqv6e65dqy3kfslr3j2cdh4enhyqeqyawyssfrl02", 
          "io_type": "Input", 
          "amount": "0x3585d2040", 
          "occupied": "0x34e62ce00", 
          "asset_info": {
            "asset_type": "CKB", 
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          }, 
          "extra": {
            "type": "Frozen"
          }, 
          "block_number": "0x342814", 
          "epoch_number": "0x708028c000ca2"
        }, 
        {
          "out_point": {
            "tx_hash": "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3", 
            "index": "0x0"
          }, 
          "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqv6e65dqy3kfslr3j2cdh4enhyqeqyawyssfrl02", 
          "io_type": "Output", 
          "amount": "0x3585a1300", 
          "occupied": "0x34e62ce00", 
          "asset_info": {
            "asset_type": "CKB", 
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          }, 
          "extra": {
            "type": "Frozen"
          }, 
          "block_number": "0x3428a9", 
          "epoch_number": "0x7080321000ca2"
        }
      ], 
      "fee": "0x30d40", 
      "burn": [ ], 
      "timestamp": "0x17d18a1e595"
    }, 
    "status": "Committed"
  }, 
  "id": 42
}
```

### Method `query_transactions`

- `query_transactions(item, asset_infos, extra, block_range, pagination, structure_type)`
  - `item`: [`JsonItem`](#type-jsonitem)
  - `asset_infos`: `Array<`[`AssetInfo>`](#type-assetinfo)`>`
  - `extra`: `"Dao"|"Cellbase"|null`
  - `block_range`: [`Range`](#type-range)`|null`
  - `pagination`: [`PaginationRequest`](#type-paginationrequest)
  - `structure_type`: `"Native"|"DoubleEntry"`
- result
  - `response`: `Array<`[`TxView`](#type-txview)`>`
  - `next_cursor`: `Uint64|null`
  - `count`: `Uint64|null`

**Usage**

To return generic transactions and pagination settings from practical searching.

**Params**

- `item` - Specify the object used to query the involved transactions.
  - If `item` is an identity, the query returns the transactions that involve addresses controlled by the identity.
  - If `item` is an address, the query returns the transactions that involve records of the address.
  - If `item` is an out point, the query returns the transactions that involve the record.
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
- `count` - The total count of transactions matching the query and ignoring pagination set. `count` can be used for calculating total pages.

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
        "type": "Address",
        "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
      },
      "asset_infos": [],
      "extra": null,
      "block_range": null,
      "pagination": {
        "cursor": null,
        "order": "asc",
        "limit": "0x1",
        "return_count": true
      },
      "structure_type": "DoubleEntry"
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "response": [
      {
        "type": "TransactionInfo", 
        "value": {
          "tx_hash": "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f", 
          "records": [
            {
              "out_point": {
                "tx_hash": "0x83bc7b8b8936b016b98dfd489a535f6cf7c3d87e60e53f83cc69e8f50c9f30fa", 
                "index": "0x0"
              }, 
              "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9", 
              "io_type": "Input", 
              "amount": "0xe8d4a51000", 
              "occupied": "0x0", 
              "asset_info": {
                "asset_type": "CKB", 
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
              }, 
              "extra": null, 
              "block_number": "0x396686", 
              "epoch_number": "0x5de0348000d61"
            }, 
            {
              "out_point": {
                "tx_hash": "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f", 
                "index": "0x0"
              }, 
              "ownership": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn", 
              "io_type": "Output", 
              "amount": "0x0", 
              "occupied": "0x0", 
              "asset_info": {
                "asset_type": "UDT", 
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
              }, 
              "extra": null, 
              "block_number": "0x397271", 
              "epoch_number": "0x3fc0257000d63"
            }, 
            {
              "out_point": {
                "tx_hash": "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f", 
                "index": "0x0"
              }, 
              "ownership": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn", 
              "io_type": "Output", 
              "amount": "0xe8d4a50dee", 
              "occupied": "0x34e62ce00", 
              "asset_info": {
                "asset_type": "CKB", 
                "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
              }, 
              "extra": null, 
              "block_number": "0x397271", 
              "epoch_number": "0x3fc0257000d63"
            }
          ], 
          "fee": "0x212", 
          "burn": [
            {
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd", 
              "amount": "0x0"
            }
          ], 
          "timestamp": "0x17dbe7d6375"
        }
      }
    ], 
    "next_cursor": "0x39727100000007", 
    "count": "0x9"
  }, 
  "id": 42
}
```

### Method `get_account_info`

- `get_account_info(item, asset_info)`
  - `item`: [`JsonItem`](#type-jsonitem)
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
- result
  - `account_number`: `Uint32`
  - `account_address`: `string`
  - `account_type`: `"Acp"|"PwLock"`

**Usage**

To return the account information for the given item and asset information. The account number returned can be used to determine whether an item has at least one specific UDT asset account.

**Params**

- `item` - Specify the object for getting the account information.
  - If `item` is an identity, the account information corresponding to the identity will be queried.
  - If `item` is an address, the account information corresponding to the address will be queried
  - If `item`  is an unspent out point, the account information corresponding to the record will be queried.
- `asset_infos` - Specify a set of asset types for the query.

**Returns**

  - `account_number`: The number of accounts for a specific UDT asset.
  - `account_address`: The address corresponding to the account.
  - `account_type`: The type of account.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_account_info",
  "params": [
    {
      "item": {
        "type": "Address",
        "value": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
      },
      "asset_info": {
        "asset_type": "UDT",
        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
      }
    }
  ]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "account_number": "0x1", 
    "account_address": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn", 
    "account_type": "Acp"
  }, 
  "id": 42
}
```

### Method `build_adjust_account_transaction`

- `build_adjust_account_transaction(item, from, asset_info, account_number, extra_ckb, fee_rate)`
  - `item`: [`JsonItem`](#type-jsonitem)
  - `from`: `Array<`[`JsonItem`](#type-jsonitem)`>`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `account_number`: `Uint32|null`
  - `extra_ckb`: `Uint64|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)`|null`
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

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
  - If `item` is an out point, the account controlled by the identity that is behind the record will be created or recycled.
- `from` - Specify the object for providing CKB for creating asset accounts.
  - If `from` is null, the method obtains CKB from `item`.
  - The elements in the `from` array must be the same kind of enumeration.
- `asset_info` - Specify an asset type for creating asset accounts.
- `account_number` - Specify a target account number.
- `extra_ckb` - Specify the amount of extra CKB injected into an account for paying fees or other usage.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transaction of creating/recycling account.
- `script_groups` - Script groups for signing.

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
        "type": "Identity",
        "value": "00791359d5f872fc4e72185034da0becb5febce98b"
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
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
            "tx_hash": "0xec26b0f85ed839ece5f11c4c4e837ec359f5adc4420410f6453b1f6b60fb96a6", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }, 
        {
          "out_point": {
            "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }, 
        {
          "out_point": {
            "tx_hash": "0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769", 
            "index": "0x0"
          }, 
          "dep_type": "code"
        }
      ], 
      "header_deps": [ ], 
      "inputs": [
        {
          "since": "0x0", 
          "previous_output": {
            "tx_hash": "0xcf9483952f04cba2b814d029c153e9ef6ce2f1d7f63dd4024246cbbdb69fc3dd", 
            "index": "0x1"
          }
        }
      ], 
      "outputs": [
        {
          "capacity": "0x34e62ce00", 
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
        }, 
        {
          "capacity": "0x42b34256f08a", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x791359d5f872fc4e72185034da0becb5febce98b"
          }, 
          "type": null
        }
      ], 
      "outputs_data": [
        "0x00000000000000000000000000000000", 
        "0x"
      ], 
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ], 
      "hash": "0xa70b07354e20ecfce43d651f7e159a7ed16d7d2a5a818b869f0f99054540e3f7"
    }, 
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x791359d5f872fc4e72185034da0becb5febce98b"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [ ]
      }, 
      {
        "script": {
          "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4", 
          "hash_type": "type", 
          "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
        }, 
        "group_type": "Type", 
        "input_indices": [ ], 
        "output_indices": [
          "0x0"
        ]
      }
    ]
  }, 
  "id": 42
}
```

### Method `build_transfer_transaction`

- `build_transfer_transaction(asset_info, from, to, output_capacity_provider, pay_fee, fee_rate, since)`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `from`: `Array<`[`JsonItem`](#type-jsonitem)`>`
  - `to`: `Array<`[`ToInfo`](#type-toinfo)`>`
  - `output_capacity_provider`: `"From"|"To"|null`
  - `pay_fee`: `"From"|"To"|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

**Usage**

To build a raw transfer transaction and script groups for signing.

**Params**

- `asset_info` - Specify the asset type for the transfer.
- `from` - Specify the sender.
  - The elements in the array must be the same kind of enumeration.
  - If `JsonItem` is an identity, the assets of addresses controlled by the identity will be pooled.
  - If `JsonItem` is an address, the assets of unspent records of the address will be pooled.
  - If `JsonItem` is an unspent out point, the assets of the out point will be pooled.
- `to` - Specify recipient's address and transfer amount.
- `output_capacity_provider` - Specify the party that provides capacity.
  - If it is `"From"`, it means that the `from` will provides the capacity required for the transfer, and the addresses of `to` represents the corresponding lock.
  - If it is `"To"`, it means that the `to` will provides the capacity required for the transfer, and the addresses of `to` must correspond to locks with acp behavior.
  - If it is `null`, same as "To", it means that `from` will not provide the required capacity, and the addresses of `to` must correspond to locks with acp behavior.
If it is To, same as None
- `pay_fee` - Specify the account for paying the fee.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration which prevents the transaction to be mined before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `script_groups` - Script groups for signing.

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
    "from": [
      {
        "type": "Address",
        "value": "ckt1qyq90n9s00ngwhmpmymrdv8wzxm82j2xylfq2agzzj"
      }
    ],
    "to": [
      {
        "address": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70",
        "amount": "0x23f2f5080"
      }
    ],
    "output_capacity_provider": "From",
    "pay_fee": "From",
    "fee_rate": null,
    "since": {
      "flag": "Absolute",
      "type_": "BlockNumber",
      "value": "0x5b8d80"
    }
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
      "header_deps": [ ], 
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
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x57ccb07be6875f61d93636b0ee11b675494627d2"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [ ]
      }
    ]
  }, 
  "id": 42
}
```

### Method `build_simple_transfer_transaction`

- `build_simple_transfer_transaction(asset_info, from, to, fee_rate, since)`
  - `asset_info`: [`AssetInfo`](#type-assetinfo)
  - `from`: `Array<string>`
  - `to`: `Array<`[`ToInfo`](#type-toinfo)`>`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

**Usage**

To build a raw transfer transaction and script groups for signing, and infer `output_capacity_provider` based on a simple strategy.

**Params**

- `asset_info` - Specify the asset type for the transfer.
- `from` - Specify the senders' addresses. 
- `to` - Specify recipient's address and amount.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration which prevents the transaction to be mined before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `script_groups` - Script groups for signing.

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
        "amount": "0x283baec00"
      }
    ],
    "fee_rate": null,
    "since": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
      "header_deps": [ ], 
      "inputs": [
        {
          "since": "0x0", 
          "previous_output": {
            "tx_hash": "0xea8c89734bde4f83809644d451fcda7784ca5c1ed89181933bbea331de39104f", 
            "index": "0xb"
          }
        }
      ], 
      "outputs": [
        {
          "capacity": "0x283baec00", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x839f1806e85b40c13d3c73866045476cc9a8c214"
          }, 
          "type": null
        }, 
        {
          "capacity": "0x30ee750f3784df2", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
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
      "hash": "0x5cb4202fc2da05187901399658c982427b4859d07a69667bf24a5be4b41b285e"
    }, 
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [ ]
      }
    ]
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
  "params": [[
    "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr"
  ]]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
  - `from`: `Array<`[`JsonItem`](#type-jsonitem)`>`
  - `to`: `string|null`
  - `amount`: `Uint64`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

**Usage**

To build a transaction to deposit specified amount of CKB to Dao.

**Params**

- `from` - Specify the provider of the CKB for Dao deposition.
  - The elements in the array must be the same kind of enumeration.
  - If `JsonItem` is an identity, the assets of addresses controlled by the identity will be pooled.
  - If `JsonItem` is an address, the assets of unspent records of the address will be pooled.
  - If `JsonItem` is an unspent out point, the assets of the out point will be pooled.
- `to` - Specify the recipient of the deposit.
  - If `to` is null, the CKB is deposited to the `from` address.
- `amount` - Specify the amount of CKB for the deposit. The deposit amount should larger than 200 CKB.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `script_groups` - Script groups for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_deposit_transaction",
  "params": [{
    "from": [
      {
        "type": "Address",
        "value": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70"
      }
    ],
    "to": null,
    "amount": "0x4a817c800",
    "fee_rate": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
            "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f", 
            "index": "0x2"
          }, 
          "dep_type": "code"
        }, 
        {
          "out_point": {
            "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }
      ], 
      "header_deps": [ ], 
      "inputs": [
        {
          "since": "0x0", 
          "previous_output": {
            "tx_hash": "0xea8c89734bde4f83809644d451fcda7784ca5c1ed89181933bbea331de39104f", 
            "index": "0xb"
          }
        }
      ], 
      "outputs": [
        {
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
        }, 
        {
          "capacity": "0x30ee74ecf1b7190", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
          }, 
          "type": null
        }
      ], 
      "outputs_data": [
        "0x0000000000000000", 
        "0x"
      ], 
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ], 
      "hash": "0x68a9eee4e15eb4da3083a5e58ab8f229baf1027ee6a8c3d791f213a4009f0c71"
    }, 
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [ ]
      }, 
      {
        "script": {
          "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e", 
          "hash_type": "type", 
          "args": "0x"
        }, 
        "group_type": "Type", 
        "input_indices": [ ], 
        "output_indices": [
          "0x0"
        ]
      }
    ]
  }, 
  "id": 42
}
```

### Method `build_dao_withdraw_transaction`

- `build_dao_withdraw_transaction(from, fee_rate)`
  - `from`: `Array<`[`JsonItem`](#type-jsonitem)`>`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

**Usage**

To build a transaction to withdraw specified deposited CKB from DAO.

**Params**

- `from` - Specify the providers for the deposit cells and fee.
- `fee_rate` -  The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `script_groups` - Script groups for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_withdraw_transaction",
  "params": [{
    "from": [
      {
        "type": "OutPoint",
        "value": {
            "tx_hash": "0x1b9757e95346d4782767c579f1d1131ead18043154229762911f82b75119f1d6", 
            "index": "0x0"
          }
      }, {
        "type": "Address",
        "value": "ckt1qyqr79tnk3pp34xp92gerxjc4p3mus2690psf0dd70"
      }
    ],
    "fee_rate": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-mainnet.ckbapp.dev/0.4
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
            "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f", 
            "index": "0x2"
          }, 
          "dep_type": "code"
        }, 
        {
          "out_point": {
            "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }
      ], 
      "header_deps": [
        "0xc286c6ba57156f5457247dbb42b9d2599d93fd47c2cf4776e5410b70d559bb41"
      ], 
      "inputs": [
        {
          "since": "0x0", 
          "previous_output": {
            "tx_hash": "0x1b9757e95346d4782767c579f1d1131ead18043154229762911f82b75119f1d6", 
            "index": "0x0"
          }
        }, 
        {
          "since": "0x0", 
          "previous_output": {
            "tx_hash": "0xea8c89734bde4f83809644d451fcda7784ca5c1ed89181933bbea331de39104f", 
            "index": "0xb"
          }
        }
      ], 
      "outputs": [
        {
          "capacity": "0x25ff7a600", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0xc8f77049fe93a1f452716edc4a87000406a9ce56"
          }, 
          "type": {
            "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e", 
            "hash_type": "type", 
            "args": "0x"
          }
        }, 
        {
          "capacity": "0x30ee753773338e7", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
          }, 
          "type": null
        }
      ], 
      "outputs_data": [
        "0x03c90f0000000000", 
        "0x"
      ], 
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", 
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ], 
      "hash": "0x2f22299e53395b166572d5278d8451c22b7819d32294371b74c26dbfbbda21ac"
    }, 
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0xc8f77049fe93a1f452716edc4a87000406a9ce56"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [ ]
      }, 
      {
        "script": {
          "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e", 
          "hash_type": "type", 
          "args": "0x"
        }, 
        "group_type": "Type", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [
          "0x0"
        ]
      }, 
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x3f1573b44218d4c12a91919a58a863be415a2bc3"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x1"
        ], 
        "output_indices": [ ]
      }
    ]
  }, 
  "id": 42
}
```

### Method `build_dao_claim_transaction`

- `build_dao_claim_transaction(from, to, fee_rate)`
  - `from`: [`JsonItem`](#type-jsonitem)
  - `to`: `string|null`
  - `fee_rate`: `Uint64|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

**Usage**

To build a transaction to claim specified withdrawing CKB from DAO.

**Params**

- `from` - Specify the provider for the withdrawing cells.
- `to` - Specify the recipient of the claim.
  - If `to` is null, the CKB is claim to the `from` address.
- `fee_rate` -  The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.

**Returns**

- `tx_view` - The raw transfer transaction.
- `script_groups` - Script groups for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_claim_transaction",
  "params": [{
    "from": {
      "type": "Address",
      "value": "ckt1qyqzqfj8lmx9h8vvhk62uut8us844v0yh2hsnqvvgc"
    },
    "fee_rate": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
            "tx_hash": "0x8f8c79eb6671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f", 
            "index": "0x2"
          }, 
          "dep_type": "code"
        }, 
        {
          "out_point": {
            "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }
      ], 
      "header_deps": [
        "0x4ae8b8ffd838f7ee8ba8d100cc9c33eb6f2c52e3c5b486c8dce79e5ffa669753", 
        "0x4d04a3679e02da060ed74d4672429edc69885bdc6a51977af72d5ac17dc2e17a", 
        "0x2709f048a9a686344008b3089fce67f4a8fa8b7438aa8f764155d626941a0916", 
        "0x3cead765c114f2bb718751914a9a731079c0a0f79963e84939bf46d7bab8680d", 
        "0x097caae3bc3f8170f38378e06f5a686674b2347cb02d9f672701a5b832bf011c"
      ], 
      "inputs": [
        {
          "since": "0x2007080295000dc2", 
          "previous_output": {
            "tx_hash": "0x05fd615326f3ccb56076b1d45c1acc33b8e605aa1e327fdc2fc54d17f54dce1c", 
            "index": "0x1"
          }
        }, 
        {
          "since": "0x2007080295000dc2", 
          "previous_output": {
            "tx_hash": "0x05fd615326f3ccb56076b1d45c1acc33b8e605aa1e327fdc2fc54d17f54dce1c", 
            "index": "0x2"
          }
        }, 
        {
          "since": "0x200708053f000d6f", 
          "previous_output": {
            "tx_hash": "0x05fd615326f3ccb56076b1d45c1acc33b8e605aa1e327fdc2fc54d17f54dce1c", 
            "index": "0x3"
          }
        }, 
        {
          "since": "0x2007080457000d6f", 
          "previous_output": {
            "tx_hash": "0x05fd615326f3ccb56076b1d45c1acc33b8e605aa1e327fdc2fc54d17f54dce1c", 
            "index": "0x4"
          }
        }, 
        {
          "since": "0x20070800a0000d6f", 
          "previous_output": {
            "tx_hash": "0x05fd615326f3ccb56076b1d45c1acc33b8e605aa1e327fdc2fc54d17f54dce1c", 
            "index": "0x5"
          }
        }
      ], 
      "outputs": [
        {
          "capacity": "0x22f7b3dac7", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x202647fecc5b9d8cbdb4ae7167e40f5ab1e4baaf"
          }, 
          "type": null
        }
      ], 
      "outputs_data": [
        "0x"
      ], 
      "witnesses": [
        "0x61000000100000005500000061000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000", 
        "0x1c00000010000000100000001c000000080000000000000000000000", 
        "0x1c00000010000000100000001c000000080000000200000000000000", 
        "0x1c00000010000000100000001c000000080000000300000000000000", 
        "0x1c00000010000000100000001c000000080000000400000000000000"
      ], 
      "hash": "0x1b153988b2e8dfa5422415fade5c40e9403f3e9f6f35c06d78014674c188d76a"
    }, 
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x202647fecc5b9d8cbdb4ae7167e40f5ab1e4baaf"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0", 
          "0x1", 
          "0x2", 
          "0x3", 
          "0x4"
        ], 
        "output_indices": [ ]
      }, 
      {
        "script": {
          "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e", 
          "hash_type": "type", 
          "args": "0x"
        }, 
        "group_type": "Type", 
        "input_indices": [
          "0x0", 
          "0x1", 
          "0x2", 
          "0x3", 
          "0x4"
        ], 
        "output_indices": [ ]
      }
    ]
  }, 
  "id": 42
}
```

### Method `get_spent_transaction`

- `get_spent_transaction(outpoint, view_type)`
  - `outpoint`: [`OutPoint`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-outpoint)
  - `structure_type`: `"Native"|"DoubleEntry"`
- result
  - [`TxView`](#type-txview)

**Usage**

To obtain the transaction that uses the specified outpoint as the input.

**Params**

- `outpoint` - Specify the outpoint for the query.
- `structure_type` - Specify the structure type of the returning transaction.

**Returns**

- `TxView` - The spent transaction.

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
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "type": "TransactionInfo", 
    "value": {
      "tx_hash": "0x2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed", 
      "records": [
        {
          "out_point": {
            "tx_hash": "0xb2e952a30656b68044e1d5eed69f1967347248967785449260e3942443cbeece", 
            "index": "0x1"
          }, 
          "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqvrnuvqd6zmgrqn60rnsesy23mvex5vy9q0g8hfd", 
          "io_type": "Input", 
          "amount": "0xd9ac33e984", 
          "occupied": "0x0", 
          "asset_info": {
            "asset_type": "CKB", 
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          }, 
          "extra": null, 
          "block_number": "0x2877b6", 
          "epoch_number": "0x70804bf000af6"
        }, 
        {
          "out_point": {
            "tx_hash": "0x2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed", 
            "index": "0x0"
          }, 
          "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqvrnuvqd6zmgrqn60rnsesy23mvex5vy9q0g8hfd", 
          "io_type": "Output", 
          "amount": "0x2540be400", 
          "occupied": "0x0", 
          "asset_info": {
            "asset_type": "CKB", 
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          }, 
          "extra": null, 
          "block_number": "0x296669", 
          "epoch_number": "0x7080462000b18"
        }, 
        {
          "out_point": {
            "tx_hash": "0x2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed", 
            "index": "0x1"
          }, 
          "ownership": "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq0e9jqckdkqk6gath2qraxjxcttuu76g8swvxcx3", 
          "io_type": "Output", 
          "amount": "0xd75828038f", 
          "occupied": "0x0", 
          "asset_info": {
            "asset_type": "CKB", 
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          }, 
          "extra": null, 
          "block_number": "0x296669", 
          "epoch_number": "0x7080462000b18"
        }
      ], 
      "fee": "0x1f5", 
      "burn": [ ], 
      "timestamp": "0x17bc67c4078"
    }
  }, 
  "id": 42
}
```

### Method `get_mercury_info`

- `get_mercury_info()`
- result
  - [`MercuryInfo`](#type-mercuryinfo)

**Usage**

To get the information of Mercury.

**Returns**

- `MercuryInfo` - The information of Mercury.

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
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "mercury_version": "0.4.0", 
    "ckb_node_version": "v0.103", 
    "network_type": "Testnet", 
    "enabled_extensions": [ ]
  }, 
  "id": 42
}
```

### Method `get_db_info`

- `get_db_info()`
- result
  - [`DBInfo`](#type-dbinfo)

**Usage**

To get the information of the database.

**Returns**

- `DBInfo` - The information of the database.

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
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
  "jsonrpc": "2.0", 
  "result": {
    "version": "0.4.0", 
    "db": "PostgreSQL", 
    "conn_size": 100, 
    "center_id": 0, 
    "machine_id": 0
  }, 
  "id": 42
}
```

### Method `build_sudt_issue_transaction`

- `build_sudt_issue_transaction(owner, to, output_capacity_provider, pay_fee, fee_rate, since)`
  - `owner`: `string`
  - `to`: `Array<`[`ToInfo`](#type-toinfo)`>`
  - `output_capacity_provider`: `"From"|"To"|null`
  - `pay_fee`: [`JsonItem`](#type-jsonitem)`|null`
  - `fee_rate`: `Uint64|null`
  - `since`: [`SinceConfig`](#type-sinceconfig)`|null`
- result
  - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
  - `script_groups`: `Array<`[`ScriptGroup`](#type-scriptgroup)`>`

**Usage**

To build a raw sUDT issuing transaction and script group for signing.

**Params**

- `owner` - Specify the owner address for the sUDT cell.
- `to` - Specify recipient's address, amount etc.
- `output_capacity_provider` - Specify the party that provides capacity.
  - If it is `"From"`, it means that the `from` will provides the capacity required for the transfer, and the addresses of `to` represents the corresponding lock.
  - If it is `"To"`, it means that the `to` will provides the capacity required for the transfer, and the addresses of `to` must correspond to locks with acp behavior.
  - If it is `null`, same as `"To"`, it means that `from` will not provide the required capacity, and the addresses of `to` must correspond to locks with acp behavior.
- `pay_fee` - Specify the account for paying the fee.
  - If `pay_fee` is null, the `owner` address pays the fee.
- `fee_rate` - The unit for the fee is shannon or KB. The default fee rate is 1000. 1 CKB = 10<sup>8</sup> shannons.
- `since` - Specify the since configuration which prevents the transaction to be mined before a certain block timestamp or a block number.

**Returns**

- `tx_view` - The raw transfer transaction.
- `script_groups` - Script groups for signing.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_sudt_issue_transaction",
  "params": [{
    "owner": "ckt1qyq9vxdzkgsdve78hexvgnvl66364ss05y3qqa6lgk",
    "to": [
      {
        "address": "ckt1qyqz86vx4klk6lxv62lsdxr5qlmksewp6s7q2l6x9t",
        "amount": "0x77359400"
      }
    ],
    "output_capacity_provider": "From",
    "pay_fee": null,
    "fee_rate": null,
    "since": null
  }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
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
            "tx_hash": "0x7f96858be0a9d584b4a9ea190e0420835156a6010a5fde15ffcdc9d9c721ccab", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }, 
        {
          "out_point": {
            "tx_hash": "0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37", 
            "index": "0x0"
          }, 
          "dep_type": "dep_group"
        }, 
        {
          "out_point": {
            "tx_hash": "0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769", 
            "index": "0x0"
          }, 
          "dep_type": "code"
        }
      ], 
      "header_deps": [ ], 
      "inputs": [
        {
          "since": "0x0", 
          "previous_output": {
            "tx_hash": "0x405c5ac01aec4270caa9df45f33fd9cadbc7ff4d7cbc02e738c57593f0f1f600", 
            "index": "0x1"
          }
        }
      ], 
      "outputs": [
        {
          "capacity": "0x3c5986200", 
          "lock": {
            "code_hash": "0x60d5f39efce409c587cb9ea359cefdead650ca128f0bd9cb3855348f98c70d5b", 
            "hash_type": "type", 
            "args": "0x2627ee54fd091f5723590e16271f6309d755ca6f3ea6a19921331ac8a3f7ec6bcae80ef746884eb2"
          }, 
          "type": {
            "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4", 
            "hash_type": "type", 
            "args": "0x3ea6a19921331ac8a3f7ec6bcae80ef746884eb2cbf7f8c87df721a6bc879758"
          }
        }, 
        {
          "capacity": "0x102299eb5d", 
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
            "hash_type": "type", 
            "args": "0x5619a2b220d667c7be4cc44d9fd6a3aac20fa122"
          }, 
          "type": null
        }
      ], 
      "outputs_data": [
        "0x00943577000000000000000000000000", 
        "0x"
      ], 
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ], 
      "hash": "0x90539d2c1af6244b09e94efa42d9d6506c2ffa4931489104b6d14322df924bdb"
    }, 
    "script_groups": [
      {
        "script": {
          "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8", 
          "hash_type": "type", 
          "args": "0x5619a2b220d667c7be4cc44d9fd6a3aac20fa122"
        }, 
        "group_type": "Lock", 
        "input_indices": [
          "0x0"
        ], 
        "output_indices": [ ]
      }, 
      {
        "script": {
          "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4", 
          "hash_type": "type", 
          "args": "0x3ea6a19921331ac8a3f7ec6bcae80ef746884eb2cbf7f8c87df721a6bc879758"
        }, 
        "group_type": "Type", 
        "input_indices": [ ], 
        "output_indices": [
          "0x0"
        ]
      }
    ]
  }, 
  "id": 42
}
```

### Method `get_sync_state`

- `get_sync_state()`
- result
  - [`SyncState`](#type-SyncState)

**Usage**

To get the state of synchronization.

There are 4 states. `Readonly` means synchronization is closed (set by Mercury's configuration file). Except for `Readonly`, a complete synchronization is composed of the other three, and each stage is executed in sequence: `ParallelFirstStage` -> `ParallelSecondStage` -> `Seria`. And each stage has its own way of calculating progress.

When the synchronization state is the `Serial` and the completion percentage is close to 100.0%, the synchronization is considered complete.

**Returns**

- `SyncState` - The state of synchronization.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_sync_state",
  "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
    "jsonrpc": "2.0",
    "result": {
        "type": "ParallelFirstStage",
        "value": {
            "current": "382225",
            "target": "446842",
            "progress": "85.5%"
        }
    },
    "id": 42
}
```

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_sync_state",
  "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- http://127.0.0.1:8116
```

- Response

```json
{
    "jsonrpc": "2.0",
    "result": {
        "type": "ReadOnly"
    },
    "id": 42
}
```

### Method `start_profiler`

- `start_profiler()`

**Usage**

Start profiler for generating flame graph.

Must set `is_pprof_enabled` to `true` in config.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "start_profiler",
  "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
    "jsonrpc": "2.0",
    "result": null,
    "id": 42
}
```

### Method `report_pprof`

- `report_pprof()`

**Usage**

Generate flame graph.

Must set `is_pprof_enabled` to `true` in config.

**Examples**

- Request

```shell
echo '{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "report_pprof",
  "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev/0.4
```

- Response

```json
{
    "jsonrpc": "2.0",
    "result": null,
    "id": 42
}
```

## RPC Types

### Type `JsonItem`

Fields

- `type` (Type: `"Identity"|"Address"|"OutPoint"`): Specify the type of item.
- `value` (Type: `string` | `string` | [`OutPoint`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-outpoint) ): Specify the value of item.

### Type `AssetInfo`

Fields

- `asset_type` (Type: `"CKB"` | `"UDT"`): Specify the asset type.
- `udt_hash` (Type: `string`): Specify the hash of a UDT asset.

### Type `Balance`

Fields

- `ownership` (Type: `string`): An address which represents the ownership of the balance.
- `asset_info` (Type: [`AssetInfo`](#type-assetinfo): Specify the asset type of the balance.
- `free` (Type: `Uint128`): Specify the amount of freely spendable assets.
- `occupied` (Type: `Uint128`): Specify the amount of CKB that provides capacity.
- `frozen` (Type: `Uint128`): Specify the amount of locked assets.

### Type `Range`

Fields

- `from` (Type: `Uint64`): Specify the start block number of the range.
- `to` (Type: `Uint64`): Specify the end block number of the range.

### Type `PaginationRequest`

This is a cursor-based pagination configuration.

Fields

- `cursor` (Type:`Uint64` | `null` ): Specify the beginning cursor for the query.
  - Start from the biggest cursor for descending order
  - Start from the smallest cursor for ascending order
- `order` (Type: `"Asc"` | `"Desc"`): Specify the order of the returning data.
- `limit` (Type: `Uint64` | `null` ): Specify the entry limit per page of the query.
- `return_count` (Type: `bool`): Specify whether to return the total count.

### Type `BlockInfo`

A double-entry style blockchain structure.

Fields

- `block_number` (Type: `Uint64`): Specify the block number.
- `block_hash` (Type: `string`): Specify the block hash.
- `parent_block_hash` (Type: `string`): Specify the parent block hash.
- `timestamp` (Type: `Uint64`): Specify the timestamp.
- `transactions` (Type:  `Array<`[`TransactionInfo`](#type-transactioninfo)`>`): Specify double-entry style transactions in the block.

### Type `TxView`

Fields

- `type` (Type: `"TransactionInfo"|"TransactionWithRichStatus"`): Specify the type of transaction view.
- `value` (Type: [`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithRichStatus`](#type-transactionwithrichstatus)): Specify the value of transaction view.

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

- `status` (Type: [`Status`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-status)): The transaction status, allowed values: "Pending", "Proposed" "Committed" "Unknown" and "Rejected".
- `block_hash` (Type: [`H256`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-h256) `|` `null`): Specify the block hash of the block which has committed this transaction in the canonical chain.
- `reason` (Type: `string` `|` `null`): Specify the reason why the transaction is rejected.
- `timestamp` (Type: `Uint64` `|` `null`): Specify the timestamp of the block in which the transaction is packaged.

### Type `Record`

A double-entry style structure that is designed to reflect the asset amount changes of an address in a transaction.

Fields

- `out_point` (Type: [`OutPoint`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-outpoint)): Specify the transaction out point of the record.
- `ownership` (Type: `string`): An address which represents the ownership of the record.
- `io_type` (Type: `"Input"|"Output"`): Specify record io type.
  - `Input` when the record is spent, and `Output` when the record is new.
- `amount` (Type: `Uint128`): Specify the amount changes.
- `occupied` (Type: `Uint64`): Specify the amount of CKB that provides capacity.
- `asset_info` (Type: [`AssetInfo`](#type-assetinfo)): Specify the asset type of the record.
- `extra` (Type:  [`ExtraFilter`](#type-extrafilter)`|null`): Specify extra information of the record.
- `block_number` (Type: [`BlockNumber`](#type-blocknumber)): Block number.
- `epoch_number` (Type: `Uint64`): Epoch value encoded.

### Type `ExtraFilter`

Fields

- `type` (Type: `"Dao"|"Cellbase"|"Frozen"`): Specify the type of extra filter.
- `value` (Type: [`DaoInfo`](#type-daoinfo)`|null`) : Specify the value of extra filter.

### Type `DaoInfo`

Fields

- `state`  (Type: [`DaoState`](#type-daoState)): Specify the state of a DAO operation.
- `reward` (Type: `Uint64`): Specify the accumulate reward of a DAO operation.

### Type `DaoState`

Fields

- `type` (Type: `"Deposit"|"Withdraw"`): Specify the type of dao state.
- `value` (Type: [`BlockNumber`](#type-blocknumber)`|Array`<[`BlockNumber`](#type-blocknumber)`>`) : Specify the block number of a dao state, when type is `"Deposit"`, value is a block number, in case type is `"Withdraw"`, value is size 2 array: `[block number, block number]`.

### Type `BurnInfo`

Fields

- `udt_hash` (Type: `string`):  Specify the type of burned assets.
- `amount` (Type: `Uint128`):  Specify the amount of burned asset.

### Type `ScriptGroup`

A struct for signing on a raw transaction.

Fields

- `script`  (Type: [`Script`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-script)): Describes the lock script and type script for a cell.
- `group_type`  (Type: `"Lock"|"Type"`): Group type.
- `input_indices`   (Type: `Array<Uint32>`): 
All input indices within this group.
- `output_indices`  (Type: `Array<Uint32>`):
All output indices within this group.

### Type `ToInfo`

Fields

- `address` (Type: `string`): Specify the recipient's address.
- `amount`  (Type: `Uint128`): Specify the amount of the asset received by the recipient.

### Type `SinceConfig`

The [since rule](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0017-tx-valid-since/0017-tx-valid-since.md)  is used to prevent a transaction to be mined before a certain block timestamp or a block number

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

### Type `SyncState`

Fields

- `type` (Type: `"ReadOnly"|"ParallelFirstStage"|"ParallelSecondStage"|"Serial"`) 
- `value` (Type: [`SyncProgress`](#type-syncprogress))

### Type `SyncProgress`

Fields

- `current`(Type: `string`): current number synchronized at the current stage.
- `target`(Type: `string`): target number at the current stage.
- `progress`(Type: `string`): Percentage of progress calculated based on current and target.

### Type `Uint32`

The [32-bit unsigned integer type](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-uint32) encoded as the 0x-prefixed hex string in JSON.

### Type `Uint64`

The [64-bit unsigned integer type](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-uint64) encoded as the 0x-prefixed hex string in JSON.

### Type `Uint128`

The [128-bit unsigned integer type](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-uint128) encoded as the 0x-prefixed hex string in JSON.

### Type `BlockNumber`

Consecutive [block number](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-blocknumber) starting from 0.
