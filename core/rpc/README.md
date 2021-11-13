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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "item": {
        "Address": "ckt1qypyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqjx5d7w"
      },
      "asset_infos": [],
      "tip_block_number": null
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
    "balances": [
      {
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
      }
    ],
    "tip_block_number": 2820020
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
    "block_number": 5386093,
    "block_hash": null
  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "block_number": 5386093,
    "block_hash": "0x0a9bbd09d83b398975efa1759a2e6b63e9a703b61ce28e25a7c7344a408bc9c9",
    "parent_hash": "0xe2837d0faf70adcdb47596b28a23973632567fc088da2ada35313b219c322c7c",
    "timestamp": 1631977498936,
    "transactions": [
      {
        "tx_hash": "0x03be4ab6639a7a4d389cf0716294906600fc539f9a763de3c85d4bbf4aac5d2c",
        "records": [
          {
            "id": "03be4ab6639a7a4d389cf0716294906600fc539f9a763de3c85d4bbf4aac5d2c0000000000636b62317179717a386a3338756a75346c39337766687478727261346c34757537757171766b74736a397768306a",
            "address_or_lock_hash": {
              "Address": "ckb1qyqz8j38uju4l93wfhtxrra4l4uu7uqqvktsj9wh0j"
            },
            "amount": "158353474693",
            "occupied": 6100000000,
            "asset_info": {
              "asset_type": "CKB",
              "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "status": {
              "Fixed": 5386093
            },
            "extra": "CellBase",
            "block_number": 5386093,
            "epoch_number": 1381006837813166
          }
        ],
        "fee": -158353474693,
        "burn": []
      }
    ]
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
  "params": "0xd3d9d86f7b28622bce3f057173c63afb4d894fbbeade70e05a67c6a23da1eb6c"
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "transaction": {
      "tx_hash": "0x4db90d39520c59481c434c83e9f9bd1435f7da8df67015fd8fff2a8b08d14fba",
      "records": [
        {
          "id": "86fd1b08837201b2abb993373d9feed7333154733b576661e8778248c96d509f0000000000636b62317179717279687865736573396c726b743275397177716d616535667939643330326e3871726b74757779",
          "address_or_lock_hash": {
            "Address": "ckb1qyqdmeuqrsrnm7e5vnrmruzmsp4m9wacf6vsxasryq"
          },
          "amount": "-119870000000000",
          "occupied": 10200000000,
          "asset_info": {
            "asset_type": "CKB",
            "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          },
          "status": {
            "Fixed": 4795381
          },
          "extra": {
            "Dao": {
              "state": {
                "Deposit": 4795381
              },
              "reward": 685925367479
            }
          },
          "block_number": 4795381,
          "epoch_number": 1581117937290780
        }
      ],
      "fee": 1462,
      "burn": []
    },
    "status": "Committed",
    "reject_reason": null
  }
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_transactions",
  "params": {
    "item": {
      "Identity": "001a4ff63598e43af9cd42324abb7657fa849c5bc3"
    },
    "asset_infos": [],
    "extra": null,
    "block_range": null,
    "pagination": {
      "cursor": null,
      "order": "desc",
      "limit": 50,
      "skip": null,
      "return_count": true
    },
    "structure_type": "DoubleEntry"
  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "response": [
      {
        "TransactionInfo": {
          "tx_hash": "0x88638e32403336912f8387ab5298ac3d3e1588082361d2fc0840808671467e54",
          "records": [
            {
              "id": "ecfea4bdf6bf8290d8f8186ed9f4da9b0f8fbba217600b47632f5a72ff677d4d0000000100636b743171797132377a367063636e63716c61616d6e683874746170776e32363065676e7436377373326377767a",
              "address_or_lock_hash": {
                "Address": "ckt1qyq27z6pccncqlaamnh8ttapwn260egnt67ss2cwvz"
              },
              "amount": "-99989999993000",
              "occupied": 0,
              "asset_info": {
                "asset_type": "UDT",
                "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
              },
              "status": {
                "Fixed": 2809155
              },
              "extra": null,
              "block_number": 2809155,
              "epoch_number": 1979149199608653
            }
          ],
          "fee": 953,
          "burn": [
            {
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "amount": "0"
            }
          ],
          "timestamp": 1631977498936
        }
      }
    ],
    "next_cursor": null,
    "count": 1
  }
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_adjust_account_transaction",
  "params": {
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
}
```

- Response

```json
{
  "id": 42,
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
        },
        {
          "out_point": {
            "tx_hash": "0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769",
            "index": "0x0"
          },
          "dep_type": "code"
        }
      ],
      "header_deps": [],
      "inputs": [
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0x92d571ca7077215d9d7d261c2f1dc379f023ad47b8cc4645e3f479b22dfdeb73",
            "index": "0x0"
          }
        },
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0xf3cc6def0286c11d06a261e8cbcbc1fa1b26a45dc555a00f6893c82d877f7ed4",
            "index": "0x2"
          }
        }
      ],
      "outputs": [
        {
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
        },
        {
          "capacity": "0x16b982112",
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
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "0x10000000100000001000000010000000"
      ],
      "hash": "0xecfbc52ce609685aadb1e28d00412495a0354311726412ef7fc8d3c1eb543b7c"
    },
    "signature_actions": [
      {
        "signature_location": {
          "index": 0, 
          "offset": 20
        }, 
        "signature_info": {
          "algorithm": "Secp256k1", 
          "address": "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr"
        }, 
        "hash_algorithm": "Blake2b", 
        "other_indexes_in_group": [1]
      }
    ]
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_transfer_transaction",
  "params": {
    "asset_info": {
      "asset_type": "CKB",
      "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
    },
    "from": {
      "items": [
        {
          "Address": "ckb1qyqgf9tl0ecx6an7msqllp0jfe99j64qtwcqhfsug7"
        }
      ],
      "source": "Free"
    },
    "to": {
      "to_infos": [
        {
          "address": "ckb1qyqdnwp9xvkukg3jxsh07ww99tlw7m7ttg6qfcatz0",
          "amount": "96500000000"
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
  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [
        {
          "out_point": {
            "tx_hash": "0x71a7ba8fc96349fea0ed3a5c47992e3b4084b031a42264a018e0072e8172e46c",
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
            "tx_hash": "0xdec460d6b1e09c79776929069ccc11fdca2c3fb970c85a33cd4124df2e92bb1e",
            "index": "0x10e"
          }
        }
      ],
      "outputs": [
        {
          "capacity": "0x1677d92500",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0xd9b825332dcb2232342eff39c52afeef6fcb5a34"
          },
          "type": null
        },
        {
          "capacity": "0x4a4a249300",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x84957f7e706d767edc01ff85f24e4a596aa05bb0"
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
      "hash": "0x626673dd080b885fa3adfc0dd104751a5d58ff6075c176f06a8dc562f47a261d"
    },
    "signature_actions": [
      {
        "signature_location": {
          "index": 0, 
          "offset": 20
        }, 
        "signature_info": {
          "algorithm": "Secp256k1", 
          "address": "ckb1qyqgf9tl0ecx6an7msqllp0jfe99j64qtwcqhfsug7"
        }, 
        "hash_algorithm": "Blake2b", 
        "other_indexes_in_group": []
      }
    ]
  }
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_simple_transfer_transaction",
  "params": {
    "asset_info": {
      "asset_type": "UDT",
      "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    },
    "from": [
      "ckt1qyqqtg06h75ymw098r3w0l3u4xklsj04tnsqctqrmc"
    ],
    "to": [
      {
        "address": "ckt1qyqg88ccqm59ksxp85788pnqg4rkejdgcg2qxcu2qf",
        "amount": "20"
      }
    ],
    "change": null,
    "fee_rate": null,
    "since": null
  }
}
```

- Response

```json
{
  "id": 42,
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
        },
        {
          "out_point": {
            "tx_hash": "0xec26b0f85ed839ece5f11c4c4e837ec359f5adc4420410f6453b1f6b60fb96a6",
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
      "header_deps": [],
      "inputs": [
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0x07748dbf03bc2341687e9d9c031bdcb570b1d78d7713c042b7d214632d51cf15",
            "index": "0x1"
          }
        },
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0x07748dbf03bc2341687e9d9c031bdcb570b1d78d7713c042b7d214632d51cf15",
            "index": "0x3"
          }
        },
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0x635772fb553a58ec799a8ab45f0d1b74ce8007a7f15dbab563f4251d1d6b84bf",
            "index": "0x0"
          }
        },
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0xb2e952a30656b68044e1d5eed69f1967347248967785449260e3942443cbeece",
            "index": "0x0"
          }
        },
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0xa2057bfafb53cb8c07cf9f611bbc9c3338152246b62cd2aa68646268e1b7f29d",
            "index": "0x0"
          }
        }
      ],
      "outputs": [
        {
          "capacity": "0xb282e1862a",
          "lock": {
            "code_hash": "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356",
            "hash_type": "type",
            "args": "0x839f1806e85b40c13d3c73866045476cc9a8c214"
          },
          "type": {
            "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
            "hash_type": "type",
            "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
          }
        },
        {
          "capacity": "0xbdafe7513f",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x05a1fabfa84db9e538e2e7fe3ca9adf849f55ce0"
          },
          "type": null
        },
        {
          "capacity": "0x442c3d2df",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x05a1fabfa84db9e538e2e7fe3ca9adf849f55ce0"
          },
          "type": {
            "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
            "hash_type": "type",
            "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc"
          }
        }
      ],
      "outputs_data": [
        "0x3c190000000000000000000000000000",
        "0x",
        "0x3c000000000000000000000000000000"
      ],
      "witnesses": [
        "0x10000000100000001000000010000000",
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "0x10000000100000001000000010000000",
        "0x10000000100000001000000010000000",
        "0x10000000100000001000000010000000"
      ],
      "hash": "0x59d9da10cac908a15f42955806c1e082fa2d371618000e18eafe3fc33752094f"
    },
    "signature_actions": [
      {
        "signature_location": {
          "index": 1, 
          "offset": 20
        }, 
        "signature_info": {
          "algorithm": "Secp256k1", 
          "address": "ckt1qyqqtg06h75ymw098r3w0l3u4xklsj04tnsqctqrmc"
        }, 
        "hash_algorithm": "Blake2b", 
        "other_indexes_in_group": [2,3,4]
      }
    ]
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
  "params": [
    "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr"
  ]
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": [
    "0xca9fc3cbc670e67451e920e6f57c647f529e567f"
  ]
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_deposit_transaction",
  "params": {
    "from": {
      "items": [
        {
          "Address": "ckb1qyqgf9tl0ecx6an7msqllp0jfe99j64qtwcqhfsug7"
        }
      ],
      "source": "Free"
    },
    "to": null,
    "amount": 20000000000,
    "fee_rate": null
  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [
        {
          "out_point": {
            "tx_hash": "0x71a7ba8fc96349fea0ed3a5c47992e3b4084b031a42264a018e0072e8172e46c",
            "index": "0x0"
          },
          "dep_type": "dep_group"
        },
        {
          "out_point": {
            "tx_hash": "0xe2fb199810d49a4d8beec56718ba2593b665db9d52299a0f9e6e75416d73ff5c",
            "index": "0x2"
          },
          "dep_type": "code"
        }
      ],
      "header_deps": [],
      "inputs": [
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0xdec460d6b1e09c79776929069ccc11fdca2c3fb970c85a33cd4124df2e92bb1e",
            "index": "0x10e"
          }
        }
      ],
      "outputs": [
        {
          "capacity": "0x5c19e5ef9e",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x84957f7e706d767edc01ff85f24e4a596aa05bb0"
          },
          "type": null
        },
        {
          "capacity": "0x4a817c800",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x84957f7e706d767edc01ff85f24e4a596aa05bb0"
          },
          "type": {
            "code_hash": "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e",
            "hash_type": "type",
            "args": "0x"
          }
        }
      ],
      "outputs_data": [
        "0x",
        "0x0000000000000000"
      ],
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ],
      "hash": "0x43114a4f2ccdb932370328dd749ca5f16e1c9d10d8f3faa1933431002f17024a"
    },
    "signature_actions": [
      {
        "signature_location": {
          "index": 0, 
          "offset": 20
        }, 
        "signature_info": {
          "algorithm": "Secp256k1", 
          "address": "ckb1qyqgf9tl0ecx6an7msqllp0jfe99j64qtwcqhfsug7"
        }, 
        "hash_algorithm": "Blake2b", 
        "other_indexes_in_group": []
      }
    ]
  }
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_withdraw_transaction",
  "params": {
    "from": {
      "Address": "ckb1qyqrd0su0thsfgzgts0uvqkmch8f6w85cxrqxgun25"
    },
    "pay_fee": "ckb1qyq8ze8534a9hu3fs9n03kqms84yayywz6ksflfvpk",
    "fee_rate": null
  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "cell_deps": [
        {
          "out_point": {
            "tx_hash": "0xe2fb199810d49a4d8beec56718ba2593b665db9d52299a0f9e6e75416d73ff5c",
            "index": "0x2"
          },
          "dep_type": "code"
        },
        {
          "out_point": {
            "tx_hash": "0x71a7ba8fc96349fea0ed3a5c47992e3b4084b031a42264a018e0072e8172e46c",
            "index": "0x0"
          },
          "dep_type": "dep_group"
        }
      ],
      "header_deps": [],
      "inputs": [
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0x8302bdfe2f482fc69f02fb1c870cada65b1006ee23ecc4004aec56d1bb70b553",
            "index": "0x0"
          }
        },
        {
          "since": "0x0",
          "previous_output": {
            "tx_hash": "0x978868570be4e8eb8e38e3b4464404dd87c6e6c8540ecbd113c0ae33d754371f",
            "index": "0x0"
          }
        }
      ],
      "outputs": [
        {
          "capacity": "0x3136652f65",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "hash_type": "type",
            "args": "0x7164f48d7a5bf2298166f8d81b81ea4e908e16ad"
          },
          "type": null
        },
        {
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
        }
      ],
      "outputs_data": [
        "0x",
        "0xeb38190000000000"
      ],
      "witnesses": [
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "0x55000000100000005500000055000000410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      ],
      "hash": "0x103eb9a56bc1427bc96bdf61bb4008a21ec184459352921398f0b9ec85a1ab50"
    },
    "signature_actions": [
      {
        "signature_location": {
          "index": 0, 
          "offset": 20
        }, 
        "signature_info": {
          "algorithm": "Secp256k1", 
          "address": "ckb1qyq8ze8534a9hu3fs9n03kqms84yayywz6ksflfvpk"
        }, 
        "hash_algorithm": "Blake2b", 
        "other_indexes_in_group": []
      }, 
      {
        "signature_location": {
          "index": 1, 
          "offset": 20
        }, 
        "signature_info": {
          "algorithm": "Secp256k1", 
          "address": "ckb1qyqrd0su0thsfgzgts0uvqkmch8f6w85cxrqxgun25"
        }, 
        "hash_algorithm": "Blake2b", 
        "other_indexes_in_group": []
      }
    ]
  }
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

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_dao_claim_transaction",
  "params": {
    "from": {
      "Address": "ckt1qyqzqfj8lmx9h8vvhk62uut8us844v0yh2hsnqvvgc"
    },
    "fee_rate": 1000
  }
}
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
  - `transaction`: [`TransactionInfo`](#type-transactioninfo)`|`[`TransactionWithRichStatus`](#type-transactionwithrichstatus)

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
    "outpoint": {
      "tx_hash": "0xb2e952a30656b68044e1d5eed69f1967347248967785449260e3942443cbeece",
      "index": "0x1"
    },
    "structure_type": "DoubleEntry"
  }
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "TransactionInfo": {
      "tx_hash": "0x2c4e242e034e70a7b8ae5f899686c256dad2a816cc36ddfe2c1460cbbbbaaaed",
      "records": [
        {
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
          "epoch_number": 1979141331094262
        }
      ],
      "fee": 501,
      "burn": []
    }
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
  "params": {}
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "mercury_version": "v0.2.0-beta",
    "ckb_node_version": "v0.43.2",
    "network_type": "Testnet",
    "enabled_extensions": []
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
  "params": {}
}
```

- Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "version": "0.1.0",
    "db": "PostgreSQL",
    "conn_size": 100,
    "center_id": 0,
    "machine_id": 0
  }
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

- `cursor` (Type:`Int64`|`null` ): Specify the beginning cursor for the query.
  - If `cursor` is null, the query starts from the biggest cursor for descending order and from the smallest cursor for ascending order.
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
