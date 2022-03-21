## Start integration tests

### Preconditions

- install Ckb
- install and start PostgreSQL
- create new database "mercury-dev", if it already exists, delete it first and then create it
- create tables

```bash
psql -h localhost -U postgres -d mercury-dev -f devtools/create_table/create_table.sql
```

### Import Ckb initial data

Note: only needs to be executed once at initialization and reinitialization.

- Make sure the database is freshly initialized
- Make sure the Ckb node is freshly initialized

```bash
cd integration/dev_chain
rm -rf ./dev/data
ckb import -C dev data/ckb_dev.json
```

### Run tests

```bash
cargo test integration
```
