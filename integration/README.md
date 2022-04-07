## Start integration tests

### Preconditions

- install Ckb (compile and install with the latest `develop` branch)
- install and start PostgreSQL
- create new database `mercury-dev`, if it already exists, delete it first and then re-create it
- create tables and indexes

```bash
psql -h localhost -U postgres -d mercury-dev -f devtools/create_table/create_table.sql
```

### Import Ckb initial data

Note: only needs to be executed once at initialization and reinitialization.

```bash
cd integration
rm -rf ./dev_chain/dev/data  ./free-space
ckb import -C dev data/ckb_dev.json
```

### Run integration tests

```bash
cd integration
cargo run
```

or
 
```bash
cd integration
cargo run -- -n get_balance
```