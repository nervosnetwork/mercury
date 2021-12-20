POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_transaction_info",
  "params": [
    "0xd82e3050472d5b5f7603cb8142a57caffdcb2c20bd88577f77da23822d4d42a3"
  ]
}

HTTP/1.1 200
[Asserts]
jsonpath "$.error" exists
