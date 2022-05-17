# Mercury Setup Instructions

This guide will show you how to setup Mercury. All the steps here use Ubuntu 20.04 LTS.

Mercury can be [built from source](#building-from-source), or a [precompiled binary](#installing-a-precompiled-release) can be used. Then the [database tables](#setup-the-database-tables) must be setup, and Mercury will need to be [configured](#configure-mercury). Finally, you will need to [start Mercury](#running-mercury).

## Building From Source

### Step 1: Install Build Dependencies

Install basic dependencies.

```sh
apt install build-essential postgresql git wget curl libssl-dev pkg-config clang
```

Install Rust using [rustup](https://rustup.rs/)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Step 2: Clone the Repository

Clone the repository and checkout the tag of the version you intend to use. Using the dev or main branches may not be stable, so a tag is always recommended.

```sh
git clone https://github.com/nervosnetwork/mercury.git
cd mercury
git checkout v0.2.7
```

### Step 3: Build Mercury

```sh
cargo build --release
```

The resulting binary will be `target/release/mercury`.

## Installing a Precompiled Release

### Step 1: Install Dependencies

Install basic dependencies.

```sh
apt install postgresql git wget curl
```

### Step 2: Clone the Repository

We will not be building from source, but there are still some valuable files. Cloning the repository is the easiest way to get everything we need. Make sure to checkout the tag that matches the version of the release you downloaded.

```sh
git clone https://github.com/nervosnetwork/mercury.git
cd mercury
git checkout v0.2.7
```

### Step 3: Download and Extract a Precompiled Binary

```sh
wget https://github.com/nervosnetwork/mercury/releases/download/v0.2.7/mercury-x86_64-unknown-linux-gnu.tar.gz
tar xzf mercury-x86_64-unknown-linux-gnu.tar.gz
```

## Setup the Database Tables

### Step 1: Launch Psql as an Administrative User

Switch to the `postgres` user and launch `psql` as an administrative user.

```sh
sudo su -l postgres
psql
```

### Step 2: Create the User and Database

After launching `psql` use the following SQL commands to create a user and database.

```sql
CREATE USER mercury WITH ENCRYPTED PASSWORD 'mercury';
CREATE DATABASE mercury;
GRANT ALL PRIVILEGES ON DATABASE mercury TO mercury;
```

You can then quit psql using `\q` and then use `exit` to return to the previous user.

### Step 3: Use Psql to Create the Tables

Use `psql` to 

```sh
psql -h localhost -U mercury -f devtools/create_table/create_table.sql
```

## Configure Mercury

Configuration files are available for the mainnet and testnet.

```sh
# mainnet
nano mercury/devtools/config/mainnet_config.toml
```

```sh
# testnet
nano mercury/devtools/config/testnet_config.toml
```

Many configuration options are available, but the options below must be changed.

### Step 1: Update the Database Connection Information

Modify the database settings to match those you configured. Make sure that the `port` matches your configure PostgreSQL port (default: 5432).

```txt
db_type = "postgres"
db_host = "127.0.0.1"
db_port = 5432
db_name = "mercury"
db_user = "mercury"
password = "mercury"
```

### Step 2: Update the CKB Node RPC URI

If you are running a CKB node on the same machine it will not need to be updated. Update as needed.

```txt
ckb_uri = "http://127.0.0.1:8114"
```

### Step 3: Update the Listen URI

By default, Mercury will listen for local connections only.

```txt
listen_uri = "127.0.0.1:8116"
```

If you need Mercury to listen for connections from other machines, change it to the following.

```txt
listen_uri = "0.0.0.0:8116"
```

## Running Mercury

Running a binary you compiled from source.

```sh
target/release/mercury --config devtools/config/testnet_config.toml run
```

Running a precompiled binary.

```sh
./mercury --config devtools/config/testnet_config.toml run
```

Viewing the Mercury logs.

```sh
tail -f free-space/testnet/log/mercury.log
```
