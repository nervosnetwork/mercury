## Run integration tests for the first time


### Install CKB

- [install Ckb](https://docs.nervos.org/docs/basics/guides/get-ckb/#build-from-source) (compile and install with the latest release version)

### Init Database


- install and start PostgreSQL
- create new database `mercury-dev`, if it already exists, delete it first and then re-create it
- create tables and indexes

```bash
psql -h localhost -U postgres -d mercury-dev -f devtools/create_table/create_table.sql
```

### Init CKB

```bash
cd integration
rm -rf ./dev_chain/dev/data  ./free-space
```

### Run integration tests

```bash
cd integration
cargo run
```

or
 
```bash
cd integration
cargo run -- -t test_generate_blocks
```

If there is no new contract to be deployed to the genesis block, the previous preparation work is no longer required, and the integration test can be run directly.

## Deploy a new contract to the genesis block

Currently Mercury's integration tests are based on dev chain
, the contracts deployed on it are all implemented through the genesis block config declaration.

If you need to deploy contract scripts on the dev chain, you need to do the following:

- init database (Same as previous section)
- put the compiled contract binary into the specified location

    ```bash
    dev_chain/dev/specs/cells
    ```

- update `dev.toml`: add new script information

    ```toml
    [[genesis.system_cells]]
    file = { file = "cells/omni_lock" }
    create_type_id = true
    capacity = 200_555_0000_0000
    ```

- init CKB (Same as previous section)
- run CKB node and get transactions in genesis block

    After completing the initialization of ckb, you can start the ckb node independently.


    ```bash
    ckb run -C dev_chain/dev --skip-spec-check
    ```

    Then you can directly call CKB's RPC `get_block_by_number`.

    ```bash
    echo '{
    "id": 42,
    "jsonrpc": "2.0",
    "method": "get_block_by_number",
    "params": [
        "0x0"
    ]
    }' \
    | tr -d '\n' \
    | curl -H 'content-type: application/json' -d @- http://127.0.0.1:8114 > genesis.json
    ```

- update the existing configuration according to the genesis transactions in `devnet_config.toml`

- add new script in `devnet_config.toml`, for example:

    ```toml
    [[extension_scripts]]
    script_name = "omni_lock"
    script = '''
    {
        "args": "0x",
        "code_hash": "0xbb4469004225b39e983929db71fe2253cba1d49a76223e9e1d212cdca1f79f28",
        "hash_type": "type"
    }
    '''
    cell_dep = '''
    {
        "dep_type": "code",
        "out_point": {
            "index": "0x9",
            "tx_hash": "0x8ca16b174cdc004eca0d9de4647b38873bd7bfd305f52155f897d90b2b0b22eb"
        }
    }
    '''
    ```

    The following code is the algorithm to calculate the code hash in the above config:

    ```rust
    use ckb_types::core::ScriptHashType;
    use ckb_types::prelude::*;
    use std::str::FromStr;

    fn caculate_type_hash(code_hash: &str, args: &str, script_hash_type: ScriptHashType) -> H256 {
        let code_hash = H256::from_str(code_hash).unwrap();
        let args = H256::from_str(args).unwrap();
        let script = packed::Script::new_builder()
            .hash_type(script_hash_type.into())
            .code_hash(code_hash.pack())
            .args(ckb_types::bytes::Bytes::from(args.as_bytes().to_owned()).pack())
            .build();
        script.calc_script_hash().unpack()
    }

    #[tokio::test]
    async fn test_caculate_lock_hash() {
        let code_hash = "00000000000000000000000000000000000000000000000000545950455f4944";
        let args = "d0e6998c64e5e3ac7f04f1c05cc41c5c36af05db696333a762d4f1ef2f407468";
        let script_hash_type = ScriptHashType::Type;

        let script_hash = caculate_type_hash(code_hash, args, script_hash_type);
        println!("{:?}", script_hash.to_string());
    }
    ```

- run integration tests

    ```bash
    cd integration
    cargo run
    ```

