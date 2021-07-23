#![allow(clippy::mutable_key_type, clippy::upper_case_acronyms)]

pub mod ckb_client;
pub mod rpc_impl;
pub mod types;

mod error;
#[cfg(test)]
mod tests;

use types::{
    CollectAssetPayload, CreateAssetAccountPayload, GenericBlock, GetBalancePayload,
    GetBalanceResponse, GetGenericBlockPayload, GetGenericTransactionResponse,
    QueryGenericTransactionsPayload, QueryGenericTransactionsResponse,
    TransactionCompletionResponse, TransferPayload,
};

pub use ckb_client::CkbRpcClient;
pub use rpc_impl::{MercuryRpcImpl, CURRENT_BLOCK_NUMBER, TX_POOL_CACHE, USE_HEX_FORMAT};

use common::anyhow::Result;

use async_trait::async_trait;
use ckb_jsonrpc_types::{BlockView, LocalNode, RawTxPool, TransactionWithStatus};
use ckb_types::{core::BlockNumber, H160, H256};
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

#[rpc(server)]
pub trait MercuryRpc {
    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "get_balance",
    ///   "params": [
    ///     {
    ///       "udt_hashes": [
    ///         "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    ///       ],
    ///       "block_number": null,
    ///       "address": {
    ///         "KeyAddress": "ckb1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9srq9shl"
    ///       }
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// Response
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": {
    ///     "block_number": 4800000,
    ///     "balances": [
    ///       {
    ///         "key_address": "ckb1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9srq9shl",
    ///         "udt_hash": null,
    ///         "unconstrained": "187000000000",
    ///         "fleeting": "0",
    ///         "locked": "8700000000"
    ///       },
    ///       {
    ///         "key_address": "ckb1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9srq9shl",
    ///         "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///         "unconstrained": "569009000000",
    ///         "fleeting": "5000000000",
    ///         "locked": "0"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    #[rpc(name = "get_balance")]
    fn get_balance(&self, payload: GetBalancePayload) -> RpcResult<GetBalanceResponse>;

    #[rpc(name = "is_in_rce_list")]
    fn is_in_rce_list(&self, rce_hash: H256, addr: H256) -> RpcResult<bool>;

    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "build_transfer_transaction",
    ///   "params": [
    ///     {
    ///       "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///       "from": {
    ///         "key_addresses": {
    ///           "key_addresses": [
    ///             "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
    ///           ],
    ///           "source": "unconstrained"
    ///         }
    ///       },
    ///       "items": [
    ///         {
    ///           "key_address": {
    ///             "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
    ///             "action": "lend_by_from"
    ///           },
    ///           "amount": 100
    ///         }
    ///       ],
    ///       "change": null,
    ///       "fee_rate": null
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// Response
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": {
    ///     "tx_view":{
    ///       "version":"0x0",
    ///       "hash":"0x334bd5b9c2d3da319385ca1ed432904e4d1be3eec801bb451988776e9cdd77ca",
    ///       "cell_deps":[
    ///         {
    ///           "out_point":{
    ///             "tx_hash":"0xec26b0f85ed839ece5f11c4c4e837ec359f5adc4420410f6453b1f6b60fb96a6",
    ///             "index":"0x0"
    ///           },
    ///           "dep_type":"dep_group"
    ///         },
    ///         {
    ///           "out_point":{
    ///             "tx_hash":"0xe12877ebd2c3c364dc46c5c992bcfaf4fee33fa13eebdf82c591fc9825aab769",
    ///             "index":"0x0"
    ///           },
    ///           "dep_type":"code"
    ///         },
    ///         {
    ///           "out_point":{
    ///             "tx_hash":"0x7f96858be0a9d584b4a9ea190e0420835156a6010a5fde15ffcdc9d9c721ccab",
    ///             "index":"0x0"
    ///           },
    ///           "dep_type":"dep_group"
    ///         }
    ///       ],
    ///       "header_deps":[
    ///
    ///       ],
    ///       "inputs":[
    ///         {
    ///           "previous_output":{
    ///             "tx_hash":"0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
    ///             "index":"0x1"
    ///           },
    ///           "since":"0x0"
    ///         },
    ///         {
    ///           "previous_output":{
    ///             "tx_hash":"0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
    ///             "index":"0x2"
    ///           },
    ///           "since":"0x0"
    ///         }
    ///       ],
    ///       "outputs":[
    ///         {
    ///           "capacity":"0x3c5986200",
    ///           "type":{
    ///             "code_hash":"0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
    ///             "args":"0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc",
    ///             "hash_type":"type"
    ///           },
    ///           "lock":{
    ///             "code_hash":"0x60d5f39efce409c587cb9ea359cefdead650ca128f0bd9cb3855348f98c70d5b",
    ///             "args":"0x094bd4c6019d91202f30f6de272226eb8c24f14ee18e7b50c7a5ea144f1b1a9bfde5f964ec11e9a9",
    ///             "hash_type":"type"
    ///           }
    ///         },
    ///         {
    ///           "capacity":"0x34e62ce00",
    ///           "type":{
    ///             "code_hash":"0xc5e5dcf215925f7ef4dfaf5f4b4f105bc321c02776d6e7d52a1db3fcd9d011a4",
    ///             "args":"0x7c7f0ee1d582c385342367792946cff3767fe02f26fd7f07dba23ae3c65b28bc",
    ///             "hash_type":"type"
    ///           },
    ///           "lock":{
    ///             "code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
    ///             "args":"0xff4f9c8a43c28ed026bdee3317fec8c2e3348773",
    ///             "hash_type":"type"
    ///           }
    ///         },
    ///         {
    ///           "capacity":"0xddfb11742a",
    ///           "lock":{
    ///             "code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
    ///             "args":"0xff4f9c8a43c28ed026bdee3317fec8c2e3348773",
    ///             "hash_type":"type"
    ///           }
    ///         }
    ///       ],
    ///       "outputs_data": [
    ///         "0x64000000000000000000000000000000",
    ///         "0x380fa5d4e80000000000000000000000",
    ///         "0x"
    ///       ],
    ///       "witnesses":[
    ///
    ///       ]
    ///     },
    ///     "sigs_entry":[
    ///       {
    ///         "type":"witness_args_lock",
    ///         "index":0,
    ///         "group_len":2,
    ///         "pub_key":"ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    #[rpc(name = "build_transfer_transaction")]
    fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[rpc(name = "build_asset_account_creation_transaction")]
    fn build_asset_account_creation_transaction(
        &self,
        payload: CreateAssetAccountPayload,
    ) -> RpcResult<TransactionCompletionResponse>;

    #[rpc(name = "get_transaction_history")]
    fn get_transaction_history(&self, ident: String) -> RpcResult<Vec<TransactionWithStatus>>;

    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "register_addresses",
    ///   "params": [
    ///     "ckt1qyqg3lvz8c8k7llaw8pzxjphkygfrllumymquvc562",
    ///     "ckt1qyqyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqukw69v",
    ///     "ckt1qyqv2w7f5kuctnt03kk9l09gwuuy6wpys64s4f8vve"
    ///   ]
    /// }
    /// ```
    ///
    /// Response
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": [
    ///     "88fd823e0f6f7ffd71c2234837b11091fffcd936",
    ///     "44935ebc860de08282b0163537697e6d6d18d1fa",
    ///     "c53bc9a5b985cd6f8dac5fbca877384d382486ab"
    ///   ]
    /// }
    /// ```
    #[rpc(name = "register_addresses")]
    fn register_addresses(&self, normal_addresses: Vec<String>) -> RpcResult<Vec<H160>>;

    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "get_generic_transaction",
    ///   "params": {
    ///     "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
    ///   }
    /// }
    /// ```
    ///
    /// Response
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": {
    ///     "transaction": {
    ///       "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
    ///       "operations":[
    ///         {
    ///           "id": 0,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "amount": {
    ///             "value": "-12300000000",
    ///             "udt_hash": null, 
    ///             "status": "locked"
    ///           }
    ///         },
    ///         {
    ///           "id": 1,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "amount": {
    ///             "value": "-1000000000000",
    ///             "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///             "status": "unconstrained"
    ///           }
    ///         },
    ///         {
    ///           "id": 2,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "amount": {
    ///             "value": "-985799999361",
    ///             "udt_hash": null,
    ///             "status": "unconstrained"
    ///           }
    ///         },
    ///         {
    ///           "id": 3,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
    ///           "amount": {
    ///             "value": "16200000000",
    ///             "udt_hash": null,
    ///             "status": "locked"
    ///           }
    ///         },
    ///         {
    ///           "id": 4,
    ///           "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
    ///           "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
    ///           "amount": {
    ///             "value": "100",
    ///             "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///             "status": "unconstrained"
    ///           }
    ///         },
    ///         {
    ///           "id": 5,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "amount": {
    ///             "value": "12300000000",
    ///             "udt_hash": null,
    ///             "status": "unconstrained"
    ///           }
    ///         },
    ///         {
    ///           "id": 6,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "amount": {
    ///             "value": "999999999900",
    ///             "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///             "status": "unconstrained"
    ///           }
    ///         },
    ///         {
    ///           "id": 7,
    ///           "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "amount": {
    ///             "value": "969599998403",
    ///             "udt_hash": null,
    ///             "status": "unconstrained"
    ///           }
    ///         },
    ///       ],
    ///       "status": "committed",
    ///       "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    ///       "block_number": 2199552,
    ///       "confirmed_number": 25
    ///     }
    ///   }
    /// }
    /// ```
    #[rpc(name = "get_generic_transaction")]
    fn get_generic_transaction(&self, tx_hash: H256) -> RpcResult<GetGenericTransactionResponse>;

    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "get_generic_block",
    ///   "params": {
    ///     "block_num": 2199552,
    ///     "block_hash": None,
    ///   }
    /// }
    /// ```
    ///
    /// Response
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": {
    ///     "block_number": 2199552,
    ///     "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    ///     "parent_block_hash": "0xa2193d975f0f13702ece351ab4913ea185ad6742b450bde374349aa5462bb7c9",
    ///     "timestamp": 1627028449,
    ///     "transactions": [
    ///       {
    ///         "tx_hash": "0x26509e99f4e1f1aeb7854cb169c82d748fd96d8a43ca92d1d9abddfa0f980b3e",
    ///         "operations": [
    ///           {
    ///             "id": 0,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           }
    ///         ],
    ///         "status": "committed",
    ///         "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    ///         "block_number", 2199552,
    ///         "confirmed_number": 25
    ///       },
    ///       {
    ///         "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
    ///         "operations":[
    ///           {
    ///             "id": 0,
    ///             "key_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
    ///             "normal_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
    ///             "amount": {
    ///               "value": "111036537582",
    ///               "udt_hash": null,
    ///               "status": "locked"
    ///             }
    ///           },
    ///           {
    ///             "id": 1,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "-1000000000000",
    ///               "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 2,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "-985799999361",
    ///               "udt_hash": null,
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 3,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
    ///             "amount": {
    ///               "value": "16200000000",
    ///               "udt_hash": null,
    ///               "status": "locked"
    ///             }
    ///           },
    ///           {
    ///             "id": 4,
    ///             "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
    ///             "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
    ///             "amount": {
    ///               "value": "100",
    ///               "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 5,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "12300000000",
    ///               "udt_hash": null,
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 6,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "999999999900",
    ///               "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 7,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "969599998403",
    ///               "udt_hash": null,
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///         ],
    ///         "status": "committed",
    ///         "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    ///         "block_number": 2199552,
    ///         "confirmed_number": 25
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    #[rpc(name = "get_generic_block")]
    fn get_generic_block(&self, payload: GetGenericBlockPayload) -> RpcResult<GenericBlock>;

    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "build_asset_collection_transaction",
    ///   "params": {
    ///     "udt_hash": null,
    ///     "from_address": {
    ///       "key_addresses": {
    ///         "key_addresses": [
    ///           "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           "ckt1qyqg3lvz8c8k7llaw8pzxjphkygfrllumymquvc562",
    ///           "ckt1qyqyfy67hjrqmcyzs2cpvdfhd9lx6mgc68aqukw69v",
    ///           "ckt1qyqv2w7f5kuctnt03kk9l09gwuuy6wpys64s4f8vve",
    ///           "ckt1qyqprhkpl4fkl585shcauausjhjwz360hwxqy5rr28"
    ///         ],
    ///         "source": "unconstrained"
    ///       }
    ///     },
    ///     "to": {
    ///       "key_addresses": {
    ///         "key_addresses": "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr",
    ///         "action": "pay_by_from"
    ///       }
    ///     },
    ///     "fee_paid_by": "ckt1qyq8jy6e6hu89lzwwgv9qdx6p0kttl4uax9s79m0mr",
    ///     "fee_rate": null
    ///   }
    /// }
    /// ```
    ///
    /// Response
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": {
    ///
    ///   }
    /// }
    /// ```
    #[rpc(name = "build_asset_collection_transaction")]
    fn build_asset_collection_transaction(
        &self,
        payload: CollectAssetPayload,
    ) -> RpcResult<TransactionCompletionResponse>;


    ///
    /// ## Examples
    ///
    /// Request
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "method": "query_generic_transactions",
    ///   "params": {
    ///     "address": {
    ///       "KeyAddress": "0xckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz"
    ///     },
    ///     "udt_hashes": [
    ///       "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
    ///     ],
    ///     "from_block": 2900000,
    ///     "to_block": null,
    ///     "limit": 10,
    ///     "offset": null,
    ///     "order": null,
    ///   }
    /// }
    /// ```
    ///
    /// Response
    ///
    /// ```json
    /// {
    ///   "id": 42,
    ///   "jsonrpc": "2.0",
    ///   "result": {
    ///     "txs": [
    ///       {
    ///         "tx_hash": "0x26509e99f4e1f1aeb7854cb169c82d748fd96d8a43ca92d1d9abddfa0f980b3e",
    ///         "operations": [
    ///           {
    ///             "id": 0,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///           }
    ///         ],
    ///         "status": "committed",
    ///         "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    ///         "block_number", 2199552,
    ///         "confirmed_number": 2312
    ///       },
    ///       {
    ///         "tx_hash": "0xa77e51ec201e48e10eedd9c983afcb0d317c46537866536cfa4fe9070da6e24e",
    ///         "operations":[
    ///           {
    ///             "id": 0,
    ///             "key_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
    ///             "normal_address": "ckt1qyqd5eyygtdmwdr7ge736zw6z0ju6wsw7rssu8fcve",
    ///             "amount": {
    ///               "value": "111036537582",
    ///               "udt_hash": null,
    ///               "status": "locked"
    ///             }
    ///           },
    ///           {
    ///             "id": 1,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "-1000000000000",
    ///               "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 2,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "-985799999361",
    ///               "udt_hash": null,
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 3,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
    ///             "amount": {
    ///               "value": "16200000000",
    ///               "udt_hash": null,
    ///               "status": "locked"
    ///             }
    ///           },
    ///           {
    ///             "id": 4,
    ///             "key_address": "ckt1qypwrrnm2rr6t6s5fud34xlauhukfmq3ax5sekdnnt",
    ///             "normal_address": "ckt1q3sdtuu7lnjqn3v8ew02xkwwlh4dv5x2z28shkwt8p2nfruccux4hcvw0dgv0f02z383kx5mlhjlje8vz856ncvw0dgv0f02z383kx5mlhjlje8vz856jkl5z44",
    ///             "amount": {
    ///               "value": "100",
    ///               "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 5,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "12300000000",
    ///               "udt_hash": null,
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 6,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "999999999900",
    ///               "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd",
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///           {
    ///             "id": 7,
    ///             "key_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "normal_address": "ckt1qyq07nuu3fpu9rksy677uvchlmyv9ce5saesf60hfz",
    ///             "amount": {
    ///               "value": "969599998403",
    ///               "udt_hash": null,
    ///               "status": "unconstrained"
    ///             }
    ///           },
    ///         ],
    ///         "status": "committed",
    ///         "block_hash": "0x873ebbd1a6243d0ae412220c753069a09172ce530a7be0cae46e5f7fff3a1d31",
    ///         "block_number": 2199552,
    ///         "confirmed_number": 25
    ///       }
    ///     ],
    ///     "total_count": 1,
    ///     "next_offset": 1,
    ///   }
    /// }
    /// ```
    #[rpc(name = "query_generic_transactions")]
    fn query_generic_transactions(
        &self,
        payload: QueryGenericTransactionsPayload,
    ) -> RpcResult<QueryGenericTransactionsResponse>;
}

#[async_trait]
pub trait CkbRpc {
    async fn local_node_info(&self) -> Result<LocalNode>;

    async fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool>;

    async fn get_transactions(
        &self,
        hashes: Vec<H256>,
    ) -> Result<Vec<Option<TransactionWithStatus>>>;

    async fn get_block_by_number(
        &self,
        block_number: BlockNumber,
        use_hex_format: bool,
    ) -> Result<Option<BlockView>>;

    async fn get_block(&self, block_hash: H256, use_hex_format: bool) -> Result<Option<BlockView>>;
}
