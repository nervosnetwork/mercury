## About Mercury

Mercury is a tool that handles applications development on [CKB](https://github.com/nervosnetwork/ckb). 
Analogously, CKB in [Nervos](https://www.nervos.org/) ecosystem is the Linux kernel and Mercury is Ubuntu. 
Mercury is the service layer providing interfaces for CKB. 
The support for CKB core interfaces and other practical functionalities of Mercury can significantly reduce the workload for developers. 
For developing wallets applications, Mercury has the interface to get balance of an address and the interface to assemble transactions for transferring CKBytes, sUDT or xUDT. 
For exchanges scenarios, Mercury provides the functions like aggregating digital assets and fetching blocks.

Mercury is the bridge between CKB and applications. 
It provides useful RPC services for DApps that are built upon [Lumos](https://github.com/nervosnetwork/lumos) and applications such as wallets and exchanges that are built upon ckb-sdk ([java](https://github.com/nervosnetwork/ckb-sdk-java) /[go](https://github.com/nervosnetwork/ckb-sdk-go)). 
Mercury, on the other side, fetches data from CKB, processes the data and implements efficient functions based on the core interfaces of CKB.

![Mercury 架构](https://user-images.githubusercontent.com/32355308/126034305-b7bef7d5-c52c-498b-94c4-115690223a88.png)

So far, Mercury has implemented a series of practical interfaces for wallets and exchanges applications. 
More new features will be developed consistently.

## Usage

There are three ways to use Mercury.

### 1. Quick Experience

The Mercury official provides public servers for quick experience of Mercury. 
The request url for mainnet is https://Mercury-mainnet.ckbapp.dev/ , for testnet is https://Mercury-testnet.ckbapp.dev/ .

For example, you can use the following command to call Mercury api methods.

```shell
$ echo '{
    "id": 1234,
    "jsonrpc": "2.0",
    "method": "get_block_info",
    "params": [{
        "block_number": 10000, 
        "block_hash": null
    }]
}' \
| tr -d '\n' \
| curl -H 'content-type: application/json' -d @- https://Mercury-testnet.ckbapp.dev
```

#### Attention
Public servers do not guarantee high availability and high performance. 
If you want to use Mercury in a production project, please deploy and run Mercury by yourself.

### 2. Run Mercury Locally

- Step 1. Run a ckb node. If you already have a running one, skip this step.
  - run a [mainnet node](https://docs.nervos.org/docs/basics/guides/mainnet)
  - run a [testnet node](https://docs.nervos.org/docs/basics/guides/testnet)
- Step 2. Edit `mainnet_config.toml` or `testnet_config.toml` according to mainnet or testnet. These config files are located in `./devtools/config/`.
- Step 3. Download the latest version of Mercury from the [release page](https://github.com/nervosnetwork/Mercury/releases).
- Step 4. Run Mercury.
```shell
## mainnet
$ Mercury -c devtools/config/mainnet_config.toml run
## testnet
$ Mercury -c devtools/config/testnet_config.toml run
```

#### Recommended Hardware

2 Cores - 4G Memory - 50G Disk and above.

#### Synchronization Duration Expectation

If Mercury connects a synced ckb node, it takes about 5-7 hours to catch up mainnet tip or 10-14 hours to catch up testnet tip.

### 3. Run Mercury via Docker

- Step 1. Edit `docker_compose_config.toml` according to your set. This config file is located in `./devtools/config/`.

- Step 2. Edit `./docker-compose.yml` to modify the runtime environment of ckb.

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

2 Cores - 4G Memory - 50G Disk and above.

#### Synchronization Duration Expectation

The docker environment runs ckb node and Mercury from the genesis block. So it takes about 12-15 hours to catch up mainnet tip or 24-30 hours to catch up testnet tip.

## SDK Support

For now, two SDKs have supported Mercury: [ckb-sdk-java](https://github.com/nervosnetwork/ckb-sdk-java) and [ckb-sdk-go](https://github.com/nervosnetwork/ckb-sdk-go).

## License [![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Fnervosnetwork%2Fckb.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Fnervosnetwork%2Fckb?ref=badge_shield)

Mercury is released under the terms of the MIT license. See [COPYING](COPYING) for more information or see [https://opensource.org/licenses/MIT](https://opensource.org/licenses/MIT).

## Development Process

The `main` branch is regularly built and tested. It is considered already production ready; The `dev` branch is the work branch to merge new features, and it's not stable. The CHANGELOG is available in [Releases](https://github.com/nervosnetwork/Mercury/releases) in the `main` branch.

## Minimum Supported Rust Version policy (MSRV)

The crate `Mercury`'s minimum supported rustc version is 1.55.0.

---

## Documentations

- [Mercury API Documentation](https://github.com/nervosnetwork/Mercury/blob/main/core/rpc/README.md)

