POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_db_info",
  "params": [],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.db" == "PostgreSQL"
