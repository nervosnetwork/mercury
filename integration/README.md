## Start integration tests

### Preconditions

- install Ckb
- install and start PostgreSQL
- create database "mercury-dev"

```bash
psql -h localhost -U postgres -d mercury-dev -f devtools/create_table/create_table.sql
```

### Import Ckb initial data

Note: only needs to be executed once at initialization and reinitialization.

```bash
cd integration/dev_chain
ckb import -C dev data/ckb_dev.json
```

### Run tests

```bash
cargo test integration
```
