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

Mercury needs [rust](https://www.rust-lang.org/) version above 1.55. And download mercury from the [release page](https://github.com/nervosnetwork/mercury/releases).

## Usage

### Edit Config File

There are two config files corresponding to mainnet and testnet located in `./devtools/config/`. The meaning of each config item shown below.



| config item                  | meaning                                           | default value           |
| ---------------------------- | ------------------------------------------------- | ----------------------- |
| center_id                    | The data center id.                               | null                    |
| machine_id                   | The machine id.                                   | null                    |
| indexer_mode                 | Use indexer mode or not.                          | null                    |
| need_sync                    | Need synchronization parallelly or not.           | true                    |
| rpc_thread_number            | The number of threads allocated to rpc.           | 2                       |
| flush_tx_pool_cache_interval | Flush transaction pool cache interval.            | 300                     |
|                              |                                                   |                         |
| db_config                    |                                                   |                         |
| max_connection               | Max db pool connection count.                     | null                    |
| db_type                      | Database type.                                    | null                    |
| db_host                      | The database host.                                | null                    |
| db_port                      | The database port.                                | null                    |
| db_name                      | The database name.                                | null                    |
| db_user                      | The database user.                                | null                    |
| password                     | The database password.                            | null                    |
| db_log_level                 | The database log level.                           | null                    |
| cellbase_maturity            | The epoch required for cellbase maturity.         | 4                       |
| cheque_timeout               | The epoch that reciever should claim cheque cell. | 6                       |
|                              |                                                   |                         |
| network_config               |                                                   |                         |
| network_type                 | The Ckb type that mercury connected.              | "ckb"                   |
| ckb_uri                      | The Ckb node uri.                                 | "http://127.0.0.1:8114" |
| listen_uri                   | The mercury listening uri.                        | "127.0.0.1:8116"        |
|                              |                                                   |                         |
| sync_config                  |                                                   |                         |
| sync_block_batch_size        | The block batch size in synchronization.          | null                    |
| max_task_count               | The maximum task count in thread pool.            | null                    |
|                              |                                                   |                         |
| log_config                   |                                                   |                         |
| log_level                    | The mercury log level.                            | "INFO"                  |
| log_path                     | The path where the log file is stored.            | "console"               |
| use_split_file               | Split log file or not.                            | false                   |
|                              |                                                   |                         |
| builtin_scripts              | The builtin script information.                   | null                    |

### Run Mercury

#### 1. Run a ckb node, skip if you have a running node

- run a [mainnet node](https://docs.nervos.org/docs/basics/guides/mainnet)
- run a [testnet node](https://docs.nervos.org/docs/basics/guides/testnet)

#### 2. Edit the config file
Edit the database config in config file. If you want to run via Docker, you should also edit the docker-compose config file.

#### 3. Run a mercury rpc server, wait for syncing

##### Run via local
- connect a ckb mainnet node

```shell
$ mercury -c devtools/config/mainnet_config.toml run
```

- connect a ckb testnet node

```shell
$ mercury -c devtools/config/testnet_config.toml run
```

##### Run via Docker
###### Running mercury development environment

- step1
Modify mercury to synchronize the execution environment of ckb.

- step2
Modify the runtime environment of ckb like this:

```yml
environment:
    CKB_NETWORK: mainnet
```

or

```yml
environment:
    CKB_NETWORK: testnet
```

- step3
```shell
$ docker-compose up -d
```

###### Run a mercury application via docker

- step1
```shell
$ docker build -t mercury .
```

- step2
```shell
$ docker run -d -p 8116:8116 -v {user_config_path}:/app/devtools/config mercury:latest
```

#### 3. Call mercury rpc via ckb-sdk ([java](https://github.com/nervosnetwork/ckb-sdk-java) , [go](https://github.com/nervosnetwork/ckb-sdk-go))
