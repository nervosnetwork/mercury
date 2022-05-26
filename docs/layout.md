# Mercury layout

```sh
.
├── apm
│   ├── tracing
│   └── tracing-derive
├── common
│   ├── address.rs
│   ├── hash.rs
│   ├── lazy.rs
│   ├── lib.rs
│   └── utils.rs
├── core
│   ├── ckb-client
│   ├── cli
│   ├── rpc
│   │   ├── rpc-core
│   │   ├── rpc-types
│   │   └── rpc-utility
│   ├── service
│   ├── storage
│   └── synchronization
├── db
│   ├── xsql
│   └── xsql-test
├── devtools
│   ├── config
│   ├── create_table
│   └── test_data
├── docs
│   ├── config.md
│   ├── layout.md
│   ├── migration.md
│   └── setup.md
├── logger
├── protocol
│   ├── db.rs
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
- `logger` Contains the mercury structured logger.
- `protocol` Contains the protocol traits and structs.
- `src` Contains main packages
