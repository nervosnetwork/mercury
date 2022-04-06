# Migration instruction

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
