# Integration-test for mercury

## Quick start for RPC integration test

### 1. Install `hurl`
This RPC integration-test uses [`hurl`](https://hurl.dev/docs/samples.html), a convenient CLI tools for calling and testing HTTP requests.

[Here](https://hurl.dev/docs/installation.html) is the installation documentation for `hurl`.

### 2. Run tests

The test files are named as `*.hurl`. Each test file has ONE JSONRPC HTTP call.

Execute the command below to run all tests under `./rpc`. All test data are in testnet and your must specify the `mercury_testnet_host`.

```bash
hurl --variable mercury_testnet_host="<your_mercury_testnet_host>" ./**/*.hurl --test
```

Output

```
./rpc/get_balance/get_balance_ok_address_all.hurl: RUNNING [1/3]
./rpc/get_balance/get_balance_ok_address_all.hurl: SUCCESS
./rpc/get_balance/get_balance_ok_address_ckb.hurl: RUNNING [2/3]
./rpc/get_balance/get_balance_ok_address_ckb.hurl: SUCCESS
./rpc/get_balance/get_balance_ok_address_udt.hurl: RUNNING [3/3]
./rpc/get_balance/get_balance_ok_address_udt.hurl: SUCCESS
--------------------------------------------------------------------------------
Executed:  3
Succeeded: 3 (100.0%)
Failed:    0 (0.0%)
Duration:  1924ms
```


If you want to see what the response looks like for a single test, just run the command below

```bash
hurl --variable mercury_testnet_host="<your_mercury_testnet_host>" ./rpc/get_balance/get_balance_ok_address_udt.hurl | jq
```
output
```json
{
  "jsonrpc": "2.0",
  "result": {
    "balances": [
      {
        "ownership": {
          "type": "Address",
          "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
        },
        "asset_info": {
          "asset_type": "UDT",
          "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
        },
        "free": "60",
        "occupied": "0",
        "freezed": "0",
        "claimable": "0"
      }
    ],
    "tip_block_number": 3767610
  },
  "id": 100
}
```


If you want to ignore assert and just want to print RPC response, just run the command below:


```bash
hurl --variable mercury_testnet_host="<your_mercury_testnet_host>" ./rpc/get_balance/get_balance_ok_address_udt.hurl --ignore-asserts | jq
```

## Contributing

Writing hurl file is very easy. Just check the existent tests under [`test_testnet`](./test_testnet) and you will know how to write.

For more please check the [samples in hurl official site](https://hurl.dev/docs/samples.html).
