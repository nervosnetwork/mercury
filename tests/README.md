Mercury integration test

- Run all test cases: `MERCURY_TESTNET_HOST=http://127.0.0.1:8116 cargo test`
- Run all test cases in single file :`MERCURY_TESTNET_HOST=http://127.0.0.1:8116 cargo test get_balance::`
- Run specified test case: `MERCURY_TESTNET_HOST=http://127.0.0.1:8116 cargo test get_balance::test_address_ckb`
