# Mercury

Mercury is an rpc service used to support CKB / [sUDT](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0025-simple-udt/0025-simple-udt.md) token / [xUDT](https://talk.nervos.org/t/rfc-extensible-udt/5337) token management.
Developers can easily get balance, construct transfer transaction and create udt wallet with [mercury-sdk](https://github.com/nervosnetwork/ckb-sdk-java/tree/develop/ckb-mercury-sdk).

## Installation

Mercury needs [rust](https://www.rust-lang.org/) version above 1.52.1.

### Clone & Build

```shell
git clone https://github.com/nervosnetwork/mercury.git && cd mercury
cargo build --release
```

## Usage

### Edit Config File

There are two config files corresponding to mainnet and testnet located in `./devtools/config/`. The meaning of each config item shown below.

| config item       | meaning                                           | default value           |
| ----------------- | ------------------------------------------------- | ----------------------- |
| network_type      | The Ckb type that mercury connected.              | "ckb"                   |
| log_level         | The mercury log level.                            | "INFO"                  |
| store_path        | The path where the mercury data is stored.        | "./free-space/db"       |
| snapshot_path     | The path where the DB snapshot is stored.         | "./free-space/snapshot" |
| log_path          | The path where the log file is stored.            | "console"               |
| snapshot_interval | Mercury DB snapshot interval block number.        | 5000                    |
| rpc_thread_number | The number of threads allocated to rpc.           | 2                       |
| cellbase_maturity | The epoch required for cellbase maturity.         | 4                       |
| cheque_since      | The epoch that reciever should claim cheque cell. | 6                       |
| ckb_uri           | The Ckb node uri.                                 | "http://127.0.0.1:8114" |
| listen_uri        | The mercury listening uri.                        | "127.0.0.1:8116"        |
| extensions_config | The config of the enabled extensions.             | null                    |

### Run Mercury

#### 1. Run a ckb node, skip if you have a running node

- run a [mainnet node](https://docs.nervos.org/docs/basics/guides/mainnet)
- run a [testnet node](https://docs.nervos.org/docs/basics/guides/testnet)

#### 2. Run a mercury rpc server, wait for syncing

- connect a ckb mainnet node

```shell
./target/release/mercury -c devtools/config/mainnet_config.toml run
```

- connect a ckb testnet node

```shell
./target/release/mercury -c devtools/config/testnet_config.toml run
```

#### 3. Call [mercury-sdk](https://github.com/nervosnetwork/ckb-sdk-java/tree/develop/ckb-mercury-sdk) in your project

### Rollback

If you want to rollback, you can use `reset` command.

```shell
./target/release/mercury -c devtools/config/mainnet_config.toml reset -h rollback_to_height
# or
./target/release/mercury -c devtools/config/testnet_config.toml reset -h rollback_to_height
```
