# Mercury JSON-RPC Protocols

- [Core Concept](#core-concept)
  * [Key Address and Normal Address](#key-address-and-normal-address)
  * [Actions, Asset Accounts, Token Category and Source](#actions--asset-accounts--token-category-and-source)
  * [General Blockchain Data Structure](#general-blockchain-data-structure)
- [RPC](#rpc)
  * [Method `get_balance`](#method--get-balance)
  * [Method `get_generic_block`](#method--get-generic-block)
  * [Method `get_generic_transaction`](#method--get-generic-transaction)
  * [Method `query_generic_transactions`](#method--query-generic-transactions)
  * [Method `register_addresses`](#method--register-addresses)
  * [Method `build_transfer_transaction`](#method--build-transfer-transaction)
  * [Method `build_asset_account_creation_transaction`](#method--build-asset-account-creation-transaction)
  * [Method `build_asset_collection_transaction`](#method--build-asset-collection-transaction)
- [RPC Types](#rpc-types)
  * [Type `KeyAddress`](#type--keyaddress)
  * [Type `NormalAddress`](#type--normaladdress)
  * [Type `KeyAddresses`](#type--keyaddresses)
  * [Type `NormalAddresses`](#type--normaladdresses)
  * [Type `TransferItem`](#type--transferitem)
  * [Type `ToKeyAddress`](#type--tokeyaddress)
  * [Type `Balance`](#type--balance)
  * [Type `GenericBlock`](#type--genericblock)
  * [Type `GenericTransaction`](#type--generictransaction)
  * [Type `Operation`](#type--operation)
  * [Type `Amount`](#type--amount)
  * [Type `SignatureEntry`](#type--signatureentry)

## Core Concept

Before exploring the Mercury interfaces, it is crucial to understand some of Mercury's unique concepts.

### Key Address and Normal Address

The CKB [Cell Model](https://docs.nervos.org/docs/basics/concepts/cell-model) is similar to that of [UTXO](https://en.wikipedia.org/wiki/Unspent_transaction_output) in Bitcoin's terminology.
Cell is the basic unit in CKB.
The full set of unspent cells in CKB is considered being the full state of CKB at that particular point in time.
A lock script defines the ownership of a cell.
The lock script is encoded into a format which in Mercury is referred to as a normal address.

From the user's perspective, applications such as wallets and exchanges manage one or more pairs of keys as well as the digital assets controlled by these keys.
In CKB, the combinations of a public key and different contracts will generate different normal addresses.
Some complex contracts can even assign the assets of a cell to multiple users, that is, a normal address can also correspond to a set of public keys.
In addition to normal addresses, the concept of **key addresses** is used to support applications to manage digital assets.
One key address unifies all digital assets that are managed by a public key.

The main format of key addresses is the short payload format of [Secp256k1/blake160](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0021-ckb-address-format/0021-ckb-address-format.md#short-payload-format).
This is the most commonly used address format on CKB.
A key address is the encoded format of a user’s lock script that is generated from the user’s public key based on the Secp256k1/blake160 contract.
By analyzing the lock script, Mercury can accurately assign the digital asset to the key address of the owner.
Applications such as wallets can easily aggregate and manipulate all digital assets under corresponding public keys via key addresses.

When Mercury handles a cell containing one of the following lock scripts, the key address is consistent with the normal address of the cell in terms of format and content.
1. The lock scripts, such as the multisig script, control the digital assets shared by multiple users.
2. The lock scripts with no pubkey settings can assign digital assets to anyone.
3. The user defined lock scripts are unsupported by Mercury.

### Actions, Asset Accounts, Token Category and Source

The native token for the Nervos CKB is CKByte, and CKB also supports token standards such as [sUDT](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0025-simple-udt/0025-simple-udt.md) (simple User-Defined Token) and [xUDT](https://talk.nervos.org/t/rfc-extensible-udt/5337) (Extensible User-Defined Token).
Through these standards, anyone can create and issue custom tokens on CKB.
CKB solves the problem of [state explosion](https://medium.com/@happypeter1983/what-is-blockchain-state-explosion-22dd531eeb21) through a unique [economic model](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0002-ckb/0002-ckb.md#5-economic-model) design.
This design requires a certain amount of CKBytes to store the token itself.

For CKByte, the transfer can be completed as long as the transfer amount exceeds the space requirement of a CKB native token (61 CKBytes).

For custom tokens, the following three **action**s are optional when transferring tokens for providing CKBytes to store the custom tokens:
* Provided by the recipient (PayByTo).
  There is a specific type of cell on CKB called [asset accounts](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0026-anyone-can-pay/0026-anyone-can-pay.md) for storing tokens.
  An **asset account** can only store one type of tokens. The recipient requires 142 CKBytes to maintain the asset account. If the recipient creates an asset account, the Cell can provide enough CKBytes for receiving any number of the same type of tokens.
* Provided by the payer (PayByFrom).
  If the recipient does not have an available asset account, the payer can create an asset account for the recipient and complete the transfer of custom tokens.
  While paying for the custom tokens, the payer also assumes the 142 CKBytes required to create an asset account.
* Lend by the payer (LendByFrom).
  If the recipient does not have an available asset account, the payer can also create a [temporary account](https://talk.nervos.org/t/sudt-cheque-deposit-design-and-implementation/5209).
  The **temporary account** requires 162 CKBytes, and the ownership of the temporary account belongs to the payer.
  The recipient must transfer or spend tokens from this temporary account within a certain time limit, otherwise the tokens will be returned to the payer.

In terms of the restrictions on tokens, there are three **token categories**:
- Unconstrained Tokens. There is no restriction on the tokens that can be used at any time.
- Locked Tokens. The tokens in a locked state include the following situations:
  - CKBytes are being used to create asset accounts, temporary accounts, or occupied for other purposes;
  - Tokens that are in a locked period and have not yet been unlocked (the tokens that have elapsed the lock period change to unconstrained tokens).
- Fleeting Tokens. Such tokens must be transferred within a certain period of time.
  The custom tokens in the temporary account created by the payer are fleeting tokens belonging to the recipient until the timeout, and will transform into unconstrained tokens belonging to the payer after the timeout.

Among the three token categories, unconstrained and fleeting tokens can be used as the input of a transfer operation.
The tokens of the corresponding category can be selected by specifying **Source** when transferring the tokens.

### General Blockchain Data Structure
Mercury has a general blockchain data structure ([`GenericBlock`](#type--genericblock) -> [`GenericTransaction`](#type--generictransaction) -> [`Operation`](#type--operation) -> [`Amount`](#type--amount)) that is abstracted on top of the CKB data structure.
The general data structure is used to reflect the changes in the token amount of a key address.

## RPC
### Method `get_balance`
* `get_balance(address, udt_hashes, block_number)`
  * `address`: [`KeyAddress`](#type--keyaddress)`|`[`NormalAddress`](#type--normaladdress)
  * `udt_hashes`: `Array<String | null>`
  * `block_number`: `Uint64 | null`
* result
  * `block_number`: `Uint64`
  * `balances`: `Array<`[`Balance`](#type--balance)`>`

Returns the balances of specified assets grouped by key-address that related to the address for the query.

#### Params

* `address` - Using a key address will accumulate the balance controlled by the public key corresponding to the key address.
  Using a normal address will distribute the balance of the normal address to the relative key addresses.
* `block_number` - For now, it can only set `null` for getting the balance by the latest height.
  In the future, it will support get balance for specified block height.
* `udt_hashes` - Specify the kinds of assets for the query. A `null` udt_hash means CKB.
  If the set is empty, it will return the balance of all kinds of assets owned by the specified address.

#### Returns
* `block_number` - State the height corresponding to the balance returned.
* `balances` - Show the balance in three categories grouped by key address.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "udt_hashes": [
        "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
      ],
      "block_number": null,
      "address": {
        "KeyAddress": "ckb1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9srq9shl"
      }
    }
  ]
}
```

Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "block_number": 4800000,
    "balances": [
      {
        "key_address": "ckb1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9srq9shl",
        "udt_hash": null,
        "unconstrained": "187000000000",
        "fleeting": "0",
        "locked": "8700000000"
      },
      {
        "key_address": "ckb1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9srq9shl",
        "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
        "unconstrained": "569009000000",
        "fleeting": "5000000000",
        "locked": "0"
      }
    ]
  }
}
```

### Method `get_generic_block`
* `get_generic_block(block_num, block_hash)`
  * `block_num`: `Uint64 | null`
  * `block_hash`: `string | null`
* result
  Return the [`GenericBlock`](#type--genericblock) of the specified block.

Return generic block of a specified block.

#### Params

* `block_num` - Specify the block number for querying.
* `block_hash` - Specify the block hash for querying.

#### Returns
If both `block_num` and `block_hash` are `null`, return the latest block.
If `block_num` is `null` and `block_hash` is not `null`, return the block matches `block_hash`.
If `block_num` is not `null` and `block_hash` is `null`, return the block on the canonical chain matches `block_num`.
If both `block_num` and `block_hash` are not `null`, return the block on the canonical chain both matches `block_num` and `block_hash`.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_generic_block",
  "params": {
    "block_num": 2199552,
    "block_hash": null
  }
}
```

Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "block_number": 2199552,
    "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    "parent_block_hash": "0xa2193d975f0f13702ece351ab4913ea185ad6742b450bde374349aa5462bb7c9",
    "timestamp": 1627028449,
    "transactions": [
      {
        "tx_hash": "0x26509e99f4e1f1aeb7854cb169c82d748fd96d8a43ca92d1d9abddfa0f980b3e",
        "operations": [
          {
            "id": 0,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
          }
        ],
        "status": "committed",
        "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
        "block_number": 2199552,
        "confirmed_number": 25
      },
      {
        "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
        "operations":[
          {
            "id": 0,
            "key_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
            "normal_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
            "amount": {
              "value": "111036537582",
              "udt_hash": null,
              "status": "locked"
            }
          },
          {
            "id": 1,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "-1000000000000",
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "status": "unconstrained"
            }
          },
          {
            "id": 2,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "-985799999361",
              "udt_hash": null,
              "status": "unconstrained"
            }
          },
          {
            "id": 3,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
            "amount": {
              "value": "16200000000",
              "udt_hash": null,
              "status": "locked"
            }
          },
          {
            "id": 4,
            "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
            "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
            "amount": {
              "value": "100",
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "status": "unconstrained"
            }
          },
          {
            "id": 5,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "12300000000",
              "udt_hash": null,
              "status": "unconstrained"
            }
          },
          {
            "id": 6,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "999999999900",
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "status": "unconstrained"
            }
          },
          {
            "id": 7,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "969599998403",
              "udt_hash": null,
              "status": "unconstrained"
            }
          }
        ],
        "status": "committed",
        "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
        "block_number": 2199552,
        "confirmed_number": 25
      }
    ]
  }
}
```

### Method `get_generic_transaction`
* `get_generic_transaction(tx_hash)`
  * `tx_hash`: `string`
* result
  * `transaction`: [`GenericTransaction`](#type--generictransaction)
  * `status`: ` "pending" | "proposed" | "committed" `
  * `block_hash`: `string | null`
  * `block_number`: `Uint64 | null`
  * `confirmed_number`: `Uint64 | null`

Return both the generic transaction and the status of a specified transaction hash.

#### Params

* `tx_hash` - Specify the transaction hash for querying.

#### Returns

* `transaction` - Generic transaction of the specified `tx_hash`.
* `status` - Status "pending" means the transaction is in the pool and not proposed yet.
  Status "proposed" means the transaction is in the pool and has been proposed.
  Status "committed" means the transaction has been committed to the canonical chain.
* `block_hash` - If the transaction is "committed", it will return the hash of the involving block.
* `block_number` - If the transaction is "committed", it will return the height of the involving block.
* `confirmed_number` - If the transaction is "committed", it will return the confirmed number of the involving block.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_generic_transaction",
  "params": {
    "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e"
  }
}
```

Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "transaction": {
      "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
      "operations":[
        {
          "id": 0,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "amount": {
            "value": "-12300000000",
            "udt_hash": null,
            "status": "locked"
          }
        },
        {
          "id": 1,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "amount": {
            "value": "-1000000000000",
            "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
            "status": "unconstrained"
          }
        },
        {
          "id": 2,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "amount": {
            "value": "-985799999361",
            "udt_hash": null,
            "status": "unconstrained"
          }
        },
        {
          "id": 3,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
          "amount": {
            "value": "16200000000",
            "udt_hash": null,
            "status": "locked"
          }
        },
        {
          "id": 4,
          "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
          "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
          "amount": {
            "value": "100",
            "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
            "status": "unconstrained"
          }
        },
        {
          "id": 5,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "amount": {
            "value": "12300000000",
            "udt_hash": null,
            "status": "unconstrained"
          }
        },
        {
          "id": 6,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "amount": {
            "value": "999999999900",
            "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
            "status": "unconstrained"
          }
        },
        {
          "id": 7,
          "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "amount": {
            "value": "969599998403",
            "udt_hash": null,
            "status": "unconstrained"
          }
        }
      ],
      "status": "committed",
      "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
      "block_number": 2199552,
      "confirmed_number": 25
    }
  }
}
```

### Method `query_generic_transactions`
* `query_generic_transactions(address, udt_hash, from_block, to_block, limit, offset, order)`
  * `address`: [`KeyAddress`](#type--keyaddress)`|`[`NormalAddress`](#type--normaladdress)
  * `from_block`: `Uint64 | null`
  * `to_block`: `Uint64 | null`
  * `limit`: `Uint64 | null`
  * `offset`: `Uint64 | null`
  * `order`: `"asc" | desc" | null`
* result
  * `txs`: `Array<`[`GenericTransaction`](#type--generictransaction)`>`
  * `total_count`: `Uint64`
  * `next_offset`: `Uint64`

Return generic transactions and pagination settings from practical searching.

#### Params

* `address` - Specify the address for searching.
* `from_block` - Specify the height as the start point of block iteration. The default value is `0`.
* `to_block` - Specify the height as the endpoint of block iteration. The default value is the maximum value of `Uint64`.
* `limit` - Specify the page limit of the search. The default value is `50`.
* `offset` - Specify the offset of the search. The default value is `0`.
* `order` - Specify the order of the search. The value "desc" means iterating from new to old, and the value "asc" has the opposite means. The default value is `desc`.

#### Returns

* `txs` - Return the generic transactions meets the condition.
* `total_count` - The count number of returned generic transactions.
* `next_offset` - Offset for next search.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_generic_transactions",
  "params": {
    "address": {
      "KeyAddress": "0xckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
    },
    "udt_hashes": [
      "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    ],
    "from_block": 2900000,
    "to_block": null,
    "limit": 10,
    "offset": null,
    "order": null
  }
}
```

Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "txs": [
      {
        "tx_hash": "0x26509e99f4e1f1aeb7854cb169c82d748fd96d8a43ca92d1d9abddfa0f980b3e",
        "operations": [
          {
            "id": 0,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
          }
        ],
        "status": "committed",
        "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
        "block_number": 2199552,
        "confirmed_number": 2312
      },
      {
        "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
        "operations":[
          {
            "id": 0,
            "key_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
            "normal_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
            "amount": {
              "value": "111036537582",
              "udt_hash": null,
              "status": "locked"
            }
          },
          {
            "id": 1,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "-1000000000000",
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "status": "unconstrained"
            }
          },
          {
            "id": 2,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "-985799999361",
              "udt_hash": null,
              "status": "unconstrained"
            }
          },
          {
            "id": 3,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
            "amount": {
              "value": "16200000000",
              "udt_hash": null,
              "status": "locked"
            }
          },
          {
            "id": 4,
            "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
            "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
            "amount": {
              "value": "100",
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "status": "unconstrained"
            }
          },
          {
            "id": 5,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "12300000000",
              "udt_hash": null,
              "status": "unconstrained"
            }
          },
          {
            "id": 6,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "999999999900",
              "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
              "status": "unconstrained"
            }
          },
          {
            "id": 7,
            "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
            "amount": {
              "value": "969599998403",
              "udt_hash": null,
              "status": "unconstrained"
            }
          }
        ],
        "status": "committed",
        "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
        "block_number": 2199552,
        "confirmed_number": 25
      }
    ],
    "total_count": 1,
    "next_offset": 1
  }
}
```

### Method `register_addresses`
* `register_addresses(normal_addresses)`
  * `normal_addresses`: `Array<string>`
* result
  Return an array of lock script hash of the register addresses.

Register addresses are for revealing the receiver key addresses of temporary accounts.
It is pretty helpful for exchanges that support UDT assets.
Before the exchange shows the addresses for use recharge, it should register them.
After that, the exchange could match the `key_address` in [`Operation`](#type--operation)s resulting from [`get_generic_block`](#method--get-generic-block) to check user recharge.

#### Params

* `normal_addresses` - Addresses for registering.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "register_addresses",
  "params": {
    "normal_addresses": [
      "ckt1qyqg3lvz8c8k7llaw8pzxjphkygfrllumymquvc562",
      "ckt1qyqyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqukw69v",
      "ckt1qyqv2w7f5kuctnt03kk9l09gwuuy6wpys64s4f8vve"
    ]
  }
}
```

Response
```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": [
    "88fd823e0f6f7ffd71c2234837b11091fffcd936",
    "44935ebc860de08282b0163537697e6d6d18d1fa",
    "c53bc9a5b985cd6f8dac5fbca877384d382486ab"
  ]
}
```

### Method `build_transfer_transaction`
* `build_transfer_transaction(udt_hash, from, items, change, fee_rate)`
  * `from`: [`KeyAddresses`](#type--keyaddresses)`|`[`NormalAddresses`](#type--normaladdresses)
  * `items`: `Array<`[`TransferItem`](#type--transferitem)`>`
  * `udt_hash`: `string | null`
  * `change`: `string | null`
  * `fee_rate`: `Uint64 | null`
* result
  * `tx_view`: `TxView`
  * `sigs_entry`: `Array<`[`SignatureEntry`](#type--signatureentry)`>`

Build a raw transfer transaction and signature entries for signing.

#### Params

* `from` - Specify addresses offering assets. If providing multiple addresses, they should belong to a single entity.
  Using key addresses should specify the **source** while normal addresses should not.
* `items` - Specify receivers' address and amount. Using key address should specify the **action** while normal address should not.
* `udt_hash` - Specify the kind of asset for transfer. Setting `null` means transferring CKB.
* `change` - Specify a key address for change. If setting `null`, the 1st address of `from` will be the change address.
* `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.

#### Returns

* `tx_view` - The raw transfer transaction.
* `sigs_entry` - Signature entries for signing.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_transfer_transaction",
  "params": [
    {
      "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
      "from": {
        "key_addresses": {
          "key_addresses": [
            "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
          ],
          "source": "unconstrained"
        }
      },
      "items": [
        {
          "key_address": {
            "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
            "action": "lend_by_from"
          },
          "amount": 100
        }
      ],
      "change": null,
      "fee_rate": null
    }
  ]
}
```

Response

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "tx_view":{
      "version":"0x0",
      "hash":"0x334bd5b9c2d3da319385ca1ed432904e4d1be3eec801bb451988776e9cdd77ca",
      "cell_deps":[
        {
          "out_point":{
            "tx_hash":"0xec26b0f85ed839ece5f11c4c4e837ec359f5adc4420410f6453b1f6b60fb96a6",
            "index":"0x0"
          },
          "dep_type":"dep_group"
        },
        {
          "out_point":{
            "tx_hash":"0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769",
            "index":"0x0"
          },
          "dep_type":"code"
        },
        {
          "out_point":{
            "tx_hash":"0x7f96858be0a9d584b4a9ea190e0420835156a6010a5fde15ffcdc9d9c721ccab",
            "index":"0x0"
          },
          "dep_type":"dep_group"
        }
      ],
      "header_deps":[

      ],
      "inputs":[
        {
          "previous_output":{
            "tx_hash":"0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
            "index":"0x1"
          },
          "since":"0x0"
        },
        {
          "previous_output":{
            "tx_hash":"0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
            "index":"0x2"
          },
          "since":"0x0"
        }
      ],
      "outputs":[
        {
          "capacity":"0x3c5986200",
          "type":{
            "code_hash":"0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
            "args":"0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc",
            "hash_type":"type"
          },
          "lock":{
            "code_hash":"0x60d5f39efce409c587cb9ea359cefdead650ca128f0bd9cb3855348f98c70d5b",
            "args":"0x094bd4c6019d91202f30f6de272226eb8c24f14ee18e7b50c7a5ea144f1b1a9bfde5f964ec11e9a9",
            "hash_type":"type"
          }
        },
        {
          "capacity":"0x34e62ce00",
          "type":{
            "code_hash":"0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
            "args":"0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc",
            "hash_type":"type"
          },
          "lock":{
            "code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "args":"0xff4f9c8a43c28ed026bdee3317fec8c2e3348773",
            "hash_type":"type"
          }
        },
        {
          "capacity":"0xddfb11742a",
          "lock":{
            "code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "args":"0xff4f9c8a43c28ed026bdee3317fec8c2e3348773",
            "hash_type":"type"
          }
        }
      ],
      "outputs_data": [
        "0x64000000000000000000000000000000",
        "0x380fa5d4e80000000000000000000000",
        "0x"
      ],
      "witnesses":[

      ]
    },
    "sigs_entry":[
      {
        "type":"witness_args_lock",
        "index":0,
        "group_len":2,
        "pub_key":"ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
      }
    ]
  }
}
```

### Method `build_asset_account_creation_transaction`
* `build_asset_account_creation_transaction(key_address, udt_hashes, fee_rate)`
  * `key_address`: `string`
  * `udt_hashes`: `Array<string | null>`
  * `fee_rate`: `Uint64 | null`
* result
  * `tx_view`: `TxView`
  * `sigs_entry`: `Array<`[`SignatureEntry`](#type--signatureentry)`>`

Build a raw asset account creation transaction and signature entries for signing.
It supports multiple asset account creations at once.
Users should keep enough CKB in the key address for the creation.
Each asset account would lock 142 CKB.

#### Params

* `key_address` - Specify a key address to create an asset account.
* `udt_hashes` - Specify the kinds of assets for creating asset accounts. At least one kind is needed.
* `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.

#### Returns

* `tx_view` - The raw asset account creation transaction.
* `sigs_entry` - Signature entries for signing.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_asset_account_creation_transaction",
  "params": {
    "key_address": "ckt1qyqyg2676jw02yzzg2f6y4tuyu59j4kdtg4qrrn42q",
    "udt_hashes": [
      "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    ],
    "fee_rate": 1000
  }
}
```

Response
```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "tx_view": {
      "version": "0x0",
      "hash": "0xf6bcb6f7449fdfb9ded4383edf0c3e2222d16d78548f433e01c66064863da239",
      "cell_deps": [
        {
          "out_point": {
            "tx_hash": "0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769",
            "index": "0x0"
          },
          "dep_type": "code"
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
            "tx_hash": "0xec26b0f85ed839ece5f11c4c4e837ec359f5adc4420410f6453b1f6b60fb96a6",
            "index": "0x0"
          },
          "dep_type": "dep_group"
        }
      ],
      "header_deps": [
      ],
      "inputs": [
        {
          "previous_output": {
            "tx_hash": "0xae1a1c7c41fafd10f3008666e1d1e049e396ed34d82b23356aab97ba829de906",
            "index": "0x0"
          },
          "since": "0x0"
        },
        {
          "previous_output": {
            "tx_hash": "0xdee5697161749f702e367c858f560fdbb30a6cd27541bd20703f4c9df38ee42f",
            "index": "0x0"
          },
          "since": "0x0"
        },
        {
          "previous_output": {
            "tx_hash": "0xd752772656632537c378e23bf9905dc7c3812321722c885bfbee763041f307d8",
            "index": "0x0"
          },
          "since": "0x0"
        }
      ],
      "outputs": [
        {
          "capacity": "0x34e62ce00",
          "type": {
            "code_hash": "0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
            "args": "0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc",
            "hash_type": "type"
          },
          "lock": {
            "code_hash": "0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356",
            "args": "0x442b5ed49cf510424293a2557c27285956cd5a2a",
            "hash_type": "type"
          }
        },
        {
          "capacity": "0x3adc0db19",
          "lock": {
            "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "args": "0x442b5ed49cf510424293a2557c27285956cd5a2a",
            "hash_type": "type"
          }
        }
      ],
      "outputs_data": [
        "0x00000000000000000000000000000000",
        "0x"
      ],
      "witnesses": [
      ]
    },
    "sigs_entry": [
      {
        "type": "witness_args_lock",
        "index": 0,
        "group_len": 3,
        "pub_key": "ckt1qyqyg2676jw02yzzg2f6y4tuyu59j4kdtg4qrrn42q"
      }
    ]
  }
}
```

### Method `build_asset_collection_transaction`
* `build_asset_collection_transaction(from_address, to, udt_hash, fee_paid_by, fee_rate)`
  * `from_address`: [`KeyAddresses`](#type--keyaddresses)`|`[`NormalAddresses`](#type--normaladdresses)
  * `to`: [`ToKeyAddress`](#type--tokeyaddress)`|`[`NormalAddress`](#type--normaladdress)
  * `udt_hash`: `string | null`
  * `fee_paid_by`: `string`
  * `fee_rate`: `Uint64 | null`
* result
  * `tx_view`: `TxView`
  * `sigs_entry`: `Array<`[`SignatureEntry`](#type--signatureentry)`>`

Build a raw asset collection creation transaction and signature entries for signing.
An asset collection transaction transfers all of the specified assets from the giving addresses to a designated address.

#### Params

* `from_address` - Specify addresses for asset collection. It Supports at most 1000 addresses for asset collection at once.
  Using key addresses should specify the **source** while normal addresses should not.
  In CKB collection, the `source` must be `unconstrained`. In UDT collection, the `source` must be `fleeting`.
* `to` - Specify the destination address of asset collection.
  Using key address should specify the **action** while normal address should not.
  In CKB collection, the `action` must be `pay_by_from`. In UDT collection, the `action` must be `pay_by_to`.
* `udt_hash` - Specify the kind of asset for collection.
* `fee_paid_by` - Specify a key address for paying fees. The `fee_paid_by` address must not be contained in `from_address`.
* `fee_rate` - The unit is Shannon/KB, which by default is 1000. 1 CKB = 10^8 Shannon.

#### Returns

* `tx_view` - The raw asset collection creation transaction.
* `sigs_entry` - Signature entries for signing.

#### Examples

Request

```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "build_asset_collection_transaction",
  "params": {
    "udt_hash": null,
    "from_address": {
      "key_addresses": {
        "key_addresses": [
          "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
          "ckt1qyqg3lvz8c8k7llaw8pzxjphkygfrllumymquvc562",
          "ckt1qyqyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqukw69v",
          "ckt1qyqv2w7f5kuctnt03kk9l09gwuuy6wpys64s4f8vve",
          "ckt1qyqprhkpl4fkl585shcauausjhjwz360hwxqy5rr28"
        ],
        "source": "unconstrained"
      }
    },
    "to": {
      "key_addresses": {
        "key_addresses": "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr",
        "action": "pay_by_from"
      }
    },
    "fee_paid_by": "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr",
    "fee_rate": null
  }
}
```

Response
```json
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": {
    "tx_view":{
      "version":"0x0",
      "hash":"0x180799b084164828f455b91294f6d6c793db042d89909029d00ceb6a1a9842c9",
      "cell_deps":[
        {
          "out_point":{
            "tx_hash":"0xf8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37",
            "index":"0x0"
          },
          "dep_type":"dep_group"
        }
      ],
      "header_deps":[

      ],
      "inputs":[
        {
          "previous_output":{
            "tx_hash":"0x3a5c8385f357e467b970b9211793edfb1ae1b5e098157b23e67079b12cc5c0c0",
            "index":"0x1"
          },
          "since":"0x0"
        },
        {
          "previous_output":{
            "tx_hash":"0xe1ae31fd71863c85b3bddb5b1705aaf72d3239b3f7da35be9b657333782e0ff1",
            "index":"0x0"
          },
          "since":"0x0"
        },
        {
          "previous_output":{
            "tx_hash":"0xe68ff6e69c03cfba91e2ccdbb2f9e653fe936b910473fc19837317db346e455b",
            "index":"0x3"
          },
          "since":"0x0"
        },
        {
          "previous_output":{
            "tx_hash":"0x301ca9547b50d17773d31ce75996b086de4087c00bfe38d81b9d525530c78eb9",
            "index":"0x0"
          },
          "since":"0x0"
        },
        {
          "previous_output":{
            "tx_hash":"0x5214ec503e6e397540fbdbc1dd1346e6357c16e54585abad8c3053e8dce4e3d7",
            "index":"0x2"
          },
          "since":"0x0"
        },
        {
          "previous_output":{
            "tx_hash":"0xc3935a0fae0564d8352e5a847d9899baf5f2943da2a749733c1c8c718f1e7143",
            "index":"0x0"
          },
          "since":"0x0"
        }
      ],
      "outputs":[
        {
          "capacity":"0xce3dacacfd",
          "lock":{
            "code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "args":"0x63f24464000fcd7e72e808c83bb63f41438505a2",
            "hash_type":"type"
          }
        },
        {
          "capacity":"0x2540be19f",
          "lock":{
            "code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
            "args":"0x442b5ed49cf510424293a2557c27285956cd5a2a",
            "hash_type":"type"
          }
        }
      ],
      "outputs_data":[
        "0x",
        "0x"
      ],
      "witnesses":[

      ]
    },
    "sigs_entry":[
      {
        "type":"witness_args_lock",
        "index":0,
        "group_len":1,
        "pub_key":"ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
      },
      {
        "type":"witness_args_lock",
        "index":1,
        "group_len":1,
        "pub_key":"ckt1qyqg3lvz8c8k7llaw8pzxjphkygfrllumymquvc562"
      },
      {
        "type":"witness_args_lock",
        "index":2,
        "group_len":1,
        "pub_key":"ckt1qyqyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqukw69v"
      },
      {
        "type":"witness_args_lock",
        "index":3,
        "group_len":1,
        "pub_key":"ckt1qyqv2w7f5kuctnt03kk9l09gwuuy6wpys64s4f8vve"
      },
      {
        "type":"witness_args_lock",
        "index":4,
        "group_len":1,
        "pub_key":"ckt1qyqprhkpl4fkl585shcauausjhjwz360hwxqy5rr28"
      },
      {
        "type":"witness_args_lock",
        "index":5,
        "group_len":1,
        "pub_key":"ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr"
      }
    ]
  }
}
```

## RPC Types
### Type `KeyAddress`
Specify a key address.

#### Fields
`KeyAddress` is a json object with the following fields.
* `key_address`: `string` - Key address.

### Type `NormalAddress`
Specify a normal address.

#### Fields
`NormalAddress` is a json object with the following fields.
* `NormalAddress`: `string` - Normal address.

### Type `KeyAddresses`
Specify a list of key addresses and source to spend.

#### Fields
`KeyAddresses` is a json object with the following fields.
* `key_addresses`: `Array<string>` - Key addresses.
* `source`: `"unconstrained" | "fleeting"` - Balance category to spend.

### Type `NormalAddresses`
Specify a list of normal addresses.

#### Fields
`NormalAddresses` is a json object with the following fields.
* `NormalAddresses`: `Array<string>` - Normal addresses.

### Type `TransferItem`
Specify an address for receiving specified amount of assets.

#### Fields
`TransferItem` is a json object with the following fields.
* `to`: [`ToKeyAddress`](#type--tokeyaddress)`|`[`NormalAddress`](#type--normaladdress) - Address for receiving assets.
* `amount`: `Uint128` - Receiving Amount.

### Type `ToKeyAddress`
Specify a key address and action for receiving assets.

#### Fields
`ToKeyAddress` is a json object with the following fields.
* `key_address`: `string` - Key address for receiving assets.
* `action`: `"pay_by_from" | "pay_by_to" | "lend_by_from"` - Action specified.

### Type `Balance`
Show the balance in three categories grouped by key address.

#### Fields
`Balance` is a json object with the following fields.
* `key_address`: `string` - Key address.
* `udt_hash`: `string | null` - UDT hash, `null` means CKB.
* `unconstrained`: `string` - Unconstrained balance.
* `fleeting`: `string` - Fleeting balance.
* `locked`: `string` - Locked balance.

### Type `GenericBlock`
A general blockchain structure for typical usage.

#### Fields
`GenericBlock` is a json object with the following fields.
* `block_number`: `Uint64` - Block height.
* `block_hash`: `string` - Block hash.
* `parent_block_hash`: `string` - Parent block hash.
* `timestamp`: `Uint64` - Timestamp.
* `transactions`: `Array<`[`GenericTransaction`](#type--generictransaction)`>` - Generic Transactions in the block.

### Type `GenericTransaction`
A general transaction structure for typical usage.

#### Fields
`GenericTransaction` is a json object with the following fields.
* `tx_hash`: `string` - Transaction hash.
* `operations`: `Array<`[`Operation`](#type--operation)`>` - Operations in the transaction.

### Type `Operation`
A general account update structure for typical usage.
It reflects the changes in the token amount of a key address.

#### Fields
`Operation` is a json object with the following fields.
* `id`: `Uint32` - Identify of an operation in a transaction.
* `key_address`: `string` - Key address which amounts changed.
* `normal_address`: `string` - Normal address corresponding to the amounts change.
* `amount`: [`Amount`](#type--amount) - Amount changes.

### Type `Amount`
A general amount change structure for typical usage.

#### Fields
`Amount` is a json object with the following fields.
* `value`: `string` - For an input of transaction, the value is the negative of its amount. For an output of transaction, the value equals to its amount.
* `udt_hash`: `string | null` - UDT hash, `null` means CKB.
* `status`: `"unconstrained" | "fleeting" | "locked"` - The amount status.

### Type `SignatureEntry`
A struct for signing on a raw transaction.

#### Field
`SignatureEntry` is a json object with the following fields.
* `type_`: `"witness_args_lock" | "witness_args_type"`
* `index`: `Uint`
* `group_len`: `Uint`
* `pub_key`: `string` - A key address to figure out private key for signing.
* `sig_type`: `"secp256k1"` - The signature algorithm.
