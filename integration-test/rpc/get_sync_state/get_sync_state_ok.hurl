POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_sync_state",
  "params": [],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.value" exists
jsonpath "$.result.value.current" isInteger
jsonpath "$.result.value.target" isInteger
