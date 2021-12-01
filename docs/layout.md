# Mercury layout

```sh
.
├── apm
│   ├── tracing
│   └── tracing-derive
├── common
│   ├── address.rs
│   ├── hash.rs
│   ├── lib.rs
│   └── utils.rs
├── core
│   ├── ckb-client
│   ├── cli
│   ├── inspection
│   ├── rpc
│   │   ├── rpc-core
│   │   ├── rpc-types
│   │   └── rpc-utility
│   ├── service
│   ├── storage
│   └── synchronization
├── db
│   ├── rocksdb
│   ├── xsql
│   └── xsql-test
├── devtools
│   ├── config
│   ├── create_table
│   └── test_data
├── docs
│   ├── architecture.md
│   ├── config.md
│   ├── layout.md
│   └── setup.md
├── extensions
├── logger
├── protocol
│   ├── db.rs
│   ├── extension.rs
│   └── lib.rs
└── src
    └── main.rs
```

A brief description:

- `apm` Contains the application performance monitor.
- `common` Contains utilities for mercury.
- `core` Contains implementations of module traits.
- `db` Contains the database implementation.
- `devtools` Contains scripts and configurations for better use of the this repository.
- `docs` Contains project documentations.
- `extensions` Contains the mercury extensions.
- `logger` Contains the mercury structured logger.
- `protocol` Contains the protocol traits and structs.
- `src` Contains main packages
