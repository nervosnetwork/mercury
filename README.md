# Mercury

Building on top of ckb-indexer, Mercury provides handy integration features for Nervos CKB.

## Installation

Mercury needs [rust](https://www.rust-lang.org/) version above 1.52.1.

### Clone & Build

```shell
git clone https://github.com/nervosnetwork/mercury.git && cd mercury
cargo build --release
```

### Usage

### Edit Config File

The path of the default config file is `./devtools/config/config.toml`. The meaning of each config item shown below.

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

Use `run` command to run mercury. If you don't give any subcommand, mercury consider that you want to run.

```shell
./target/release/mercury -c devtool/config/config.toml run
```

If you want to rollback, you can use `reset` command.

```shell
./target/release/mercury -c devtool/config/config.toml reset -h rollback_to_height
```
