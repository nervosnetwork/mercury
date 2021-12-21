POST {{mercury_testnet_host}}
{
  "id": 2,
  "jsonrpc": "2.0",
  "method": "get_cells_capacity",
  "params": [
    {
      "script": {
        "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        "args": "0xa3b8598e1d53e6c5e89e8acb6b4c34d3adb13f2b",
        "hash_type": "type"
      },
      "script_type": "lock"
    }
  ]
}

HTTP/1.1 200
```{"jsonrpc":"2.0","error":{"code":-32001,"message":"Custom error: Error(-10090): \"Cannot find spent transaction\""},"id":42}```
