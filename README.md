# Mercury

![GitHub release](https://img.shields.io/github/v/release/nervosnetwork/mercury)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE.md)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.56.1+-informational.svg)](https://github.com/nervosnetwork/mercury/blob/main/rust-toolchain)

## About Mercury

Mercury is a tool that handles application development on [CKB](https://github.com/nervosnetwork/ckb).
In the [Nervos](https://www.nervos.org/) ecosystem, analogically speaking, while CKB is the Linux kernel, Mercury is Ubuntu.
Mercury is the service layer providing interfaces for CKB.
The support for CKB core interfaces and other practical functionalities of Mercury can significantly reduce the workload for developers.
For developing wallet applications, Mercury offers the interfaces to get balance of an address and to assemble transactions for transferring CKBytes, sUDT or xUDT.
For exchanges, Mercury provides functions such as aggregating digital assets and fetching blocks.

Mercury is the bridge between CKB and applications. 
It provides useful RPC services for DApps that are built upon [Lumos](https://github.com/nervosnetwork/lumos) and applications that are built upon ckb-sdk ([java](https://github.com/nervosnetwork/ckb-sdk-java) /[go](https://github.com/nervosnetwork/ckb-sdk-go)), such as wallets and exchanges.
Furthermore, Mercury fetches data from CKB, processes the data, and implements efficient functions based on the core interfaces of CKB.

<img src="https://user-images.githubusercontent.com/32355308/141873786-5ac316b8-c2cc-461b-b8f6-025d025037ba.png" width="450" height="380" alt="Mercury 架构"/>

So far, Mercury has implemented a series of practical interfaces for wallet and exchange applications. More features are in development.

## Long-term Support(LTS)

The v0.2 release has been designated as the LTS release. We suggest using the 0.2 version. The migration instructions for upgrading from other versions to the latest version are [here](docs/migration.md).

## Usage

There are three ways to use Mercury.

### 1. Quick Experience

The Mercury official provides public servers for a quick experience of Mercury.

For version 0.2, The request url for mainnet is https://Mercury-mainnet.ckbapp.dev/ , for testnet is https://Mercury-testnet.ckbapp.dev/ .

For version 0.3, the request url for mainnet is https://Mercury-mainnet.ckbapp.dev/0.3 , for testnet is https://Mercury-testnet.ckbapp.dev/0.3 .

For example, you can use the following command to call a Mercury API method to view the version.

```shell
echo '{
    "id": 1234,
    "jsonrpc": "2.0",
    "method": "get_mercury_info",
    "params": []
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

**Attention**  
Public servers do not guarantee high availability and high performance. 
If you want to use Mercury in a production project, please deploy and run Mercury on yourself.

### 2. Run Mercury Locally

- Step 1. Run a CKB node. If you already have a running node, skip this step.
  - run a [mainnet node](https://docs.nervos.org/docs/basics/guides/mainnet)
  - run a [testnet node](https://docs.nervos.org/docs/basics/guides/testnet)

- Step 2. Edit `mainnet_config.toml` or `testnet_config.toml` according to mainnet or testnet. These config files are located in `./devtools/config/`. The instrcution of config file is [here](docs/config.md).

- Step 3. Download the latest version of Mercury from the [release page](https://github.com/nervosnetwork/Mercury/releases).

- Step 4. Create mercury tables if not exists.

```shell
$ psql mercury -U mercury -f ~/path/to/mercury/devtools/create_table/create_table.sql
```

- Step 5. Run Mercury.

```shell
# mainnet
$ Mercury -c ~/path/to/mercury/devtools/config/mainnet_config.toml run

# testnet
$ Mercury -c ~/path/to/mercury/devtools/config/testnet_config.toml run
```

#### Recommended Hardware

8 Cores - 16G Memory - 500G Disk and above.

If you use a standalone server to run the Postgres server, a 50G Disk is enough.

#### Expected Synchronization Duration

If Mercury connects with a synced CKB node, it takes 5-7 hours to catch up the mainnet tip or 10-14 hours to catch up the testnet tip.

### 3. Run Mercury via Docker

- Step 1. Edit `docker_compose_config.toml` according to your set. This config file is located in `./devtools/config/`.

- Step 2. Edit `./docker-compose.yml` to modify the runtime environment of CKB.

```yml
environment:
    CKB_NETWORK: mainnet
```

or

```yml
environment:
    CKB_NETWORK: testnet
```

- Step 3. Build Mercury images from the Dockerfile.

```shell
$ docker build -t Mercury .
```

- Step 4. Run Mercury via docker.

```shell
$ docker-compose up -d
```

or

```shell
$ docker run -d -p 8116:8116 -v {user_config_path}:/app/devtools/config Mercury:latest
```

#### Recommended Hardware

8 Cores - 16G Memory - 500G Disk and above.

#### Expected Synchronization Duration

The docker environment runs CKB node and Mercury from the genesis block. It takes 12-15 hours to catch up mainnet tip or 24-30 hours for testnet tip.

## SDK Support

For now, two SDKs have supported Mercury: [ckb-sdk-java](https://github.com/nervosnetwork/ckb-sdk-java) and [ckb-sdk-go](https://github.com/nervosnetwork/ckb-sdk-go).

## License [![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Fnervosnetwork%2Fckb.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Fnervosnetwork%2Fckb?ref=badge_shield)

Mercury is released under the terms of the MIT license. See [COPYING](COPYING) for more information or see [https://opensource.org/licenses/MIT](https://opensource.org/licenses/MIT).

## Development Process

The `main` branch is built and tested regularly, considered as production-ready; The `dev` branch is the work branch to merge new features, and it is not stable. The CHANGELOG is available in [Releases](https://github.com/nervosnetwork/Mercury/releases) in the `main` branch.

## Minimum Supported Rust Version policy (MSRV)

The `Mercury` crate's minimum supported rust version is 1.56.1.

---

## Documentation

- [Mercury API Documentation](core/rpc/README.md)
- [Mercury Config Documentation](docs/config.md)
- [Mercury Layout Documentation](docs/layout.md)
- [Mercury Setup Instructions](docs/setup.md)
