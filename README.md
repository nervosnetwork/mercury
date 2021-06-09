## ckb-companion

Building on top of ckb-indexer, ckb-companion provides handy integration features for Nervos CKB.

## Installation

Mercury needs [rust](https://www.rust-lang.org/) version above 1.52.1.

### Clone & Build

```shell
git clone https://github.com/nervosnetwork/mercury.git && cd mercury
cargo build --release
```

### Run mercury

Use `run` command to run mercury. If you don't give any subcommand, mercury consider that you want to run.

```shell
./target/release/mercury -c devtool/config/config.toml run
```

If you want to rollback, you can use `reset` command.

```shell
./target/release/mercury -c devtool/config/config.toml reset -h rollback_to_height
```
