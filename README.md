# Mercury

Mercury is a tool that handles applications development on [CKB](https://github.com/nervosnetwork/ckb). 
Analogously, CKB in [Nervos](https://www.nervos.org/) ecosystem is the Linux kernel and Mercury is Ubuntu. 
Mercury is the service layer providing interfaces for CKB. 
The support for CKB core interfaces and other practical functionalities of Mercury can significantly reduce the workload for developers. 
For developing wallets applications, Mercury has the interface to get balance of an address and the interface to assemble transactions for transferring CKBytes, sUDT or xUDT. 
For exchanges scenarios, Mercury provides the functions like aggregating digital assets and fetching blocks.

Mercury is the bridge between CKB and applications. 
It provides useful RPC services for DApps that are built upon [Lumos](https://github.com/nervosnetwork/lumos) and applications such as wallets and exchanges that are built upon ckb-sdk ([java](https://github.com/nervosnetwork/ckb-sdk-java) /[go](https://github.com/nervosnetwork/ckb-sdk-go)). 
Mercury, on the other side, fetches data from CKB, processes the data and implements efficient functions based on the core interfaces of CKB.

![mercury 架构](https://user-images.githubusercontent.com/32355308/126034305-b7bef7d5-c52c-498b-94c4-115690223a88.png)

So far, Mercury has implemented a series of practical interfaces for wallets and exchanges applications.
Here is the [Mercury API Documentation](https://github.com/nervosnetwork/mercury/blob/main/core/rpc/README.md). 
More new features will be developed consistently.

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
| cheque_timeout    | The epoch that reciever should claim cheque cell. | 6                       |
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

#### 3. Call mercury rpc via ckb-sdk ([java](https://github.com/nervosnetwork/ckb-sdk-java) ,[go](https://github.com/nervosnetwork/ckb-sdk-go) )

### Rollback

If you want to rollback, you can use `reset` command.

```shell
./target/release/mercury -c devtools/config/mainnet_config.toml reset -h rollback_to_height
# or
./target/release/mercury -c devtools/config/testnet_config.toml reset -h rollback_to_height
```
