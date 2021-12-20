POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_spent_transaction",
  "params": [{
    "outpoint": {
      "tx_hash": "0x1f8c79eb3671709633fe6a46de93c0fedc9c1b8a6527a18d3983879542635c9f",
      "index": "0x1"
    },
    "structure_type": "Native"
  }]
}


HTTP/1.1 200
[Asserts]
jsonpath "$.error" exists
