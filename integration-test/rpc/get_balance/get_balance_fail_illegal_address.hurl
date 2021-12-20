POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "item": {
        "type": "Address",
        "value": "ckt1q"
      },
      "asset_infos": [],
      "tip_block_number": 3767610
    }
  ],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.error" exists
