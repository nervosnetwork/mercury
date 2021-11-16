# Configuration instruction

You can see the default configuration [here](../devtools/config/docker_compose_config.toml).

## Basic configuration

### `center_id`

The center id of the server. This is used to generate the distributed unique ID through snowflake algorithm.

type: `u16`

### `machine_id`

The machine id of the server. This is used to generate the distributed unique ID through snowflake algorithm.

type: `u16`

### `indexer_mode`

If mercury is in the indexer mode, mercury will pull blocks from the CKB node and append the block data to the database. Otherwise, mercury will only handle RPC requests and do not append the block data.

type: `bool`

### `need_sync`

If this is true, mercury do will synchronize blocks parallelly when start. Otherwise, mercury will judge the necessity of synchronization parallel.

type: `bool`

### `rpc_thread_num`

The thread number allocated for RPC server.

type: `usize`

### `flush_tx_pool_cache_interval`

The millisecond interval for refreshing the transaction pool cache with the connected CKB node.

type: `u64`

### `cellbase_maturity`

The epoch number of the cellbase maturity. This is same as the config of CKB. **DO NOT CHANGE THIS UNLESS TESTING**

type: `u64`

### `cheque_since`

The epoch number of cheque cell can be withdraw. This is same as the data hard-coded in cheque lock script. **DO NOT CHANGE THIS UNLESS TESTING**

type: `u64`

## DB configuration

### `max_connections`

The maximum number of connections to database pool.

type: `u32`

### `db_type`

The database type of Mercury. Such as `postgres`, `mysql`, `sqlite`.

type: `String`

### `db_host`

The host of the database.

type: `String`

### `db_port`

The port of the database.

type: `u16`

### `db_name`

The name of the database.

type: `String`

### `db_user`

The user name of the database.

type: `String`

### `password`

The password of the database.

type: `String`

### `db_log_level`

The log level of the database, uppercase.

type: `String`

## Log configuration

### `log_level`

The log level of the mercury, uppercase.

type: `String`

### `use_split_file`

If this is true, mercury will automatically split the log file.

type: `bool`

### `log_path`

The path of log files.

type: `String`

## Network configuration

### `network_type`

The [network type](../common/src/lib.rs) of CKB node, such as `ckb`, `ckb_testnet`.

type: `String`

### `ckb_uri`

The uri of CKB node.

type: `String`

### `listen_uri`

The listening uri of mercury RPC server.

type: `String`

## Synchronization configuration

### `sync_block_batch_size`

The batch size that mercury pull from the CKB node when synchronization.

type: `usize`

### `max_task_count`

The maxiumn task count that synchronize blocks parallelly.

type: `usize`

### `builtin_script`

The built in script information.

type: `String`
