# Migration instruction

## 0.4.1 Release

No migration matters.

## 0.4.0 Release

Mercury 0.4 is a big break change compared to 0.3. The changes are focused on making Mercury rpc more generic, and removing hard bindings to specific locking scripts/type scripts(especially `cheque` script), which means better extensibility and compatibility in the future.

In addition, Mercury enhances the consistency of the rpc interface: 

- the json `integer` type is uniformly replaced by hex string
- the type enumeration values ​​are unified into the capitalized style of the first letter

To explain these break changes, there will be 2 parts: the first part focuses on changes to custom types, and the second part focuses on the changes in the rpc methods.

### Part I: RPC Types Breaking Change


#### `Uint32`, `Uint64`, `Uint128`, `BlockNumber`

The `integer` types in the input and output of all RPCs are unified into the following 4 types according to the situation:

- `Uint32` is a [32-bit unsigned integer type](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-uint32) encoded as the 0x-prefixed hex string in JSON.
- `Uint64` is a [64-bit unsigned integer type](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-uint64) encoded as the 0x-prefixed hex string in JSON.
- `Uint128` is a [128-bit unsigned integer type](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-uint128) encoded as the 0x-prefixed hex string in JSON.
- `BlockNumber` is a 64-bit unsigned integer type encoded as the 0x-prefixed hex string in JSON. See `Uint64`.

For example, `3418141` is changed to `"0x4eabe1"`.

Please see the Mercury RPC README.


#### Type `ScriptGroup`

`ScriptGroup` is completely redesigned, it replaces the `SignatureAction` in 0.3 version and earlier. They are structures used to sign raw transactions, and each raw transaction that Mercury builds is paired with an array of this type to provide information for subsequent signatures.

The problem with `SignatureAction` is that it enumerates the known signature algorithm names and hash algorithm names to guide subsequent signatures, but for new signature algorithms or hash algorithms, it needs to be manually extended.

Unlike `SignatureAction`, `ScriptGroup` is more general, it describes all script groups that need to be verified in a transaction. Users only need to find the script corresponding to their private key and sign it.

`ScriptGroup` fields:

> - `script`  (Type: [`Script`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-script)): Describes the lock script and type script for a cell.
> - `group_type`  (Type: `"Lock"|"Type"`): Group type.
> - `input_indices`   (Type: `Array<Uint32>`): All input indices within this group.
> - `output_indices`  (Type: `Array<Uint32>`): All output indices within this group.


#### Type `Balance`

`Balance` fields:

> - `ownership` (Type: `string`): An address which represents the ownership of the balance.
> - `asset_info` (Type: `AssetInfo`: Specify the asset type of the balance.
> - `free` (Type: `Uint128`): Specify the amount of freely spendable assets.
> - `occupied` (Type: `Uint128`): Specify the amount of CKB that provides capacity.
> - `frozen` (Type: `Uint128`): Specify the amount of locked assets.

- The type of the field `ownership` is changed from the `Ownership`(in 0.3 version and earlier) to `string` representing an address.
- The types of fields `free`, `occupied` and `frozen` are changed from `string` to `Uint128`, which is a 128-bit unsigned integer type encoded as the 0x-prefixed hex string in JSON. For example, `"56800000000"` is changed to `"0xd398b3800"`.
- The field `claimbale` in version 0.3 and earlier has been removed, this field is specially used to describe the `UDT` amount locked by `cheque` lock. In version 0.4, the amount of udt will be counted in the `free` field.


#### Type `Record`

`Record` fields

> - `out_point` (Type: [`OutPoint`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-outpoint)): Specify the transaction out point of the record.
> - `ownership` (Type: `string`): An address which represents the ownership of the record.
> - `io_type` (Type: `"Input"|"Output"`): Specify record io type.
>   - `Input` when the record is spent, and `Output` when the record is new.
> - `amount` (Type: `Uint128`): Specify the amount changes.
> - `occupied` (Type: `Uint64`): Specify the amount of CKB that provides capacity.
> - `asset_info` (Type: `AssetInfo`: Specify the asset type of the record.
> - `extra` (Type:  `ExtraFilter``|null`): Specify extra information of the record.
> - `block_number` (Type: `BlockNumber`): Block number.
> - `epoch_number` (Type: `Uint64`): Epoch value encoded.

- The type of the field `ownership` is changed from the `Ownership`(in 0.3 version and earlier) to `string` representing the address.
- Added `io_type` field to indicate whether `Record` is `Input` or `Output`, in 0.3 version and earlier it is represented by the sign of `amount`.
- The type of the `amount` field is changed from the `BigInt` to `Uint64`, and it is always a positive integer.
- The field "status" in 0.3 version and earlier has been removed, which is also a specialization of `cheque` script.

#### Type `ExtraFilter`

`ExtraFilter` fields

> - `type` (Type: `"Dao"|"Cellbase"|"Frozen"`): Specify the type of extra filter.
> - `value` (Type: `DaoInfo|null|null`) : Specify the value of extra filter.

- for the enumeration value in `type` filed, rename `Freeze` to `Frozen`, and `CellBase` to `Cellbase`.


#### Type `PaginationRequest`

`PaginationRequest` fields

> - `cursor` (Type:`Uint64` | `null` ): Specify the beginning cursor for the query.
>   - Start from the biggest cursor for descending order
>   - Start from the smallest cursor for ascending order
> - `order` (Type: `"Asc"` | `"Desc"`): Specify the order of the returning data.
> - `limit` (Type: `Uint64` | `null` ): Specify the entry limit per page of the query.
> - `return_count` (Type: `bool`): Specify whether to return the total count.

- The type of `cursor` is changed from `Array<Int8>` (array size is 8) to `Uint64`.
- The enumeration values ​​of `order` are unified in the style of capital letters.


#### Type `TxRichStatus`

`TxRichStatus` fields

> - `status` (Type: [`Status`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-status)): The transaction status, allowed values: `"Pending"`, `"Proposed"`, `"Committed"`, `"Unknown"` and `"Rejected"`.
> - `block_hash` (Type: [`H256`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-h256) `|` `null`): > Specify the block hash of the block which has committed this transaction in the canonical chain.
> - `reason` (Type: `string` `|` `null`): Specify the reason why the transaction is rejected.
> - `timestamp` (Type: `Uint64` `|` `null`): Specify the timestamp of the block in which the transaction is packaged.

- The values ​​enumerated in the `status` field are unified in the style of capital letters.


#### Type `BurnInfo`

`BurnInfo` fields

> - `udt_hash` (Type: `string`):  Specify the type of burned assets.
> - `amount` (Type: `Uint128`):  Specify the amount of burned asset.

- The `amount` type is unified from `string` to `Uint128`.


#### Type `SyncProgress`

`SyncProgress` fields

> - `current`(Type: `string`): current number synchronized at the current stage.
> - `target`(Type: `string`): target number at the current stage.
> - `progress`(Type: `string`): Percentage of progress calculated based on current and target.

- For human readability, the `current` and `target` fields are changed from `integer` to `string`.


#### Types deprecated  

The following types that existed in 0.3 and earlier are deprecated:

- `Source`
- `Ownership`
- `Status`
- `From`
- `To`
- `SignatureLocation`

### Part II: RPC Methods Breaking Change

#### Method `build_transfer_transaction`

> - `build_transfer_transaction(asset_info, from, to, output_capacity_provider, pay_fee, fee_rate, since)`
>   - `asset_info`: `AssetInfo`
>   - `from`: `Array<JsonItem>`
>   - `to`: `Array<ToInfo>`
>   - `output_capacity_provider`: `"From"|"To"|null`
>   - `pay_fee`: `"From"|"To"|null`
>   - `fee_rate`: `Uint64|null`
>   - `since`: `SinceConfig|null`
> - result
>   - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
>   - `script_groups`: `Array<ScriptGroup>`

- `output_capacity_provider` field replaces the `mode` filed in `To` type in 0.3 version and earlier, 
user can use `output_capacity_provider` field to implement the same transfer behavior supported by 0.3 and earlier versions, the corresponding methods as follows:

    | AssetType | transfer behavior of 0.3 version and earlier                                             | the same transfer behavior by 0.4                                                          |
    | --------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
    | `CKB`     | `Mode::HoldByFrom`: build a new output cell without type script that is the same as the lock of `to`, and has the capacity of the transferred amount. | `OutputCapacityProvider::From`:  same as left                |
    | `CKB`     | `Mode::HoldByTo`: try to find the live `acp` cell corresponding to the `to` address(Mercury needs to know this correspondence), if it exists, put it in the input of the transaction, and let it receive the `CKB` of the transferred amount as an output cell in an unsigned manner | `OutputCapacityProvider::To` or `None`: let `to ` be the same `acp` address (Need to be built in advance, Mercury will not check), then the behavior will be the same as the left |
    | `UDT`     | `Mode::HoldByFrom`: newly build `cheque` cell as output cell, its capacity is just enough, and its udt amount is the transfer amount, the `cheque` lock args sender part comes from `from`, and the receiver part comes from `to` | `OutputCapacityProvider::From`:  just let to be the same `cheque` address (Need to be built in advance, Mercury will not check), then the behavior will be the same as the left |
    | `UDT`     | `Mode::HoldByTo`:  try to find the `acp` cell corresponding to the `to` address(Mercury needs to know this correspondence), if it exists, put it in the input of the transaction, and let it receive the `UDT` of the transferred amount as an output cell in an unsigned manner | `OutputCapacityProvider::To` or `None`: let `to ` be the same `acp` address (Need to be built in advance, Mercury will not check), then the behavior will be the same as the left |
    | `UDT`     | `Mode::PayWithAcp`: build a new `acp` cell corresponding to the `to` address(Mercury needs to know this correspondence), its capacity is just enough, and its udt amount is the transfer amount, this means that `from` not only needs to pay fee but also the capacity to create this cell | `OutputCapacityProvider::From`: let `to ` be the same `acp` address (Need to be built in advance, Mercury will not check), then the behavior will be the same as the left |
- compared with "0.3 and earlier versions, `pay_fee` field is no longer a third-party address, but can be set to pay fee by `"From"` or `"To"`, the default is `"From"`, this option is only valid when transferring `CKB`.

    `from` pays the fee: alice transfers 200 to bob, alice pays fee 0.1, the final balance of alice is -200.1, and the balance of bob is +200. 
    ```
    from: alice
    to: bob
    amount: 200
    pay_fee: "From"
    ```

    `to` pays the fee: alice transfers 200 to bob, bob pays fee 0.1, the final balance of alice is -200, the balance of bob is +199.9
    ```
    from: alice
    to: bob
    amount: 200
    pay_fee: "To"
    ```
-  removed the `change` filed.


#### Method `build_dao_withdraw_transaction`

> - `build_dao_withdraw_transaction(from, fee_rate)`
>   - `from`: `Array<JsonItem>`
>   - `fee_rate`: `Uint64|null`
> - result
>   - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
>   - `script_groups`: `Array<ScriptGroup>`

- `from` field changed from type `JsonItem` to `Array<JsonItem>`, this allows more items to be passed in at the same time.
- the `pay_fee` field in 0.3 and earlier versions has been removed, which is a third-party address
  
  When we want to withdraw a deposit cell and need an address to pay fee, we can do the following：

    ```
    '{
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
    }'
    ```

#### Method `build_dao_claim_transaction`

> - `build_dao_claim_transaction(from, to, fee_rate)`
>   - `from`: `Array<JsonItem>`
>   - `to`: `string|null`
>   - `fee_rate`: `Uint64|null`
> - result
>   - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
>   - `script_groups`: `Array<ScriptGroup>`

- - `from` field changed from type `JsonItem` to `Array<JsonItem>`, this allows more items to be passed in at the same time.

#### Method `build_sudt_issue_transaction`

> - `build_sudt_issue_transaction(owner, from, to, output_capacity_provider, fee_rate, since)`
>   - `owner`: `string`
>   - `from`: `Array<JsonItem>`
>   - `to`: `Array<ToInfo>`
>   - `output_capacity_provider`: `"From"|"To"|null`
>   - `fee_rate`: `Uint64|null`
>   - `since`: `SinceConfig|null`
> - result
>   - `tx_view`: [`TransactionView`](https://github.com/nervosnetwork/ckb/blob/develop/rpc/README.md#type-transactionview)
>   - `script_groups`: `Array<ScriptGroup>`

- added `from` field, which specify the providers for capacity and fee, which must contain owner item. In this way, more items can participate in pooling ckb.
- `output_capacity_provider` field replaces the `mode` filed in `to` field in 0.3 version and earlier, 
user can use `output_capacity_provider` field to implement the same transfer behavior supported by 0.3 and earlier versions, usage is similar to rpc method `build_transfer_transaction` subsection
- the `pay_fee` field in 0.3 and earlier versions has been removed , which is a third-party address.
- the `change` filed in 0.3 and earlier versions has been removed.

#### Method `register_addresses`

> - `register_addresses(addresses)`
>   - `addresses`: `Array<string>`
> - result
>   - A list of lock script hash of the registered addresses.

**Deprecated**: 

This rpc is currently used very rarely and will be removed in a future version.


## 0.3.1 Release

No migration matters.

## 0.3.0 Release

Mercury 0.3.0 has a breaking change that modifies 3 structures in the rpc and is not fully compatible with the current sdk:

- in the `JsonItem` type, the enum value `OutPoint` replaces `Record`
- in the `Record` type, the `out_point` field replaces the `id` field
- in the `Balance` type, renamed the `freezed` field to `frozen`

## 0.2.5 Release

No migration matters.

## 0.2.4 Release

This release has two new features:

- `build_transfer_transaction` rpc supports a new mode `PayWithAcp` to transfer UDT assets
- a new rpc `get_account_info` is used to get account information

If you are using [SDK](https://github.com/nervosnetwork/mercury#sdk-support) to connect to mercury, you need to upgrade the SDK version to the new corresponding version.

## 0.2.3 Release

Mercury 0.2.3 should be paired with the latest [configuration file](https://github.com/nervosnetwork/mercury/blob/v0.2.3/devtools/config/mainnet_config.toml), which has a Breaking Change: it adds the configuration for the pw lock script.

In this version of the database creation script create_tabel.sql, the creation of three indexes has been removed, which greatly improves the database query performance: 
- namely index_indexer_cell_table_type_hash
- index_live_cell_table_type_hash
- index_cell_table_type_hash

For the database that has been established, we recommend that the above three index items be deleted manually.


Another suggestion to improve performance is to downgrade the version of the PostgreSQL database from version 14 to version 10. We found that the database query performance will also be improved.

## 0.2.2 Release

No migration matters.

## 0.2.1 Release

No migration matters.

## 0.2.0 Release

Starting from 0.2.0, any address in the return value of mercury such as `SignatureAction` and `RecordID` will be encoded as new format full address. Meanwhile, the short address and old format full address will be deprecated.

### From earlier than v0.2.0-beta.4

All versions, except for v0.2.0-beta.4, need to clear the database and resynchronize the data.

### From v0.2.0-beta.4

Upgrading from v0.2.0-beta.4 version do not need to resynchronize data. However, it should be noted that the `extra_filter` field of `Record` strcuture add a new `Freezed` type.

If you are using [SDK](https://github.com/nervosnetwork/mercury#sdk-support) to connect to mercury, you only need to upgrade the SDK version to v0.101.1, otherwise you need to adapt the `Record` structure.
