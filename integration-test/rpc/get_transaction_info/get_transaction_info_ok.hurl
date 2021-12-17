POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_transaction_info",
  "params": [
    "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3"
  ]
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.transaction" exists
jsonpath "$.result.transaction.tx_hash" == "0xd82e3050472d5b5f7603cb8141a57caffdcb2c20bd88577f77da23822d4d42a3"

# input #0
jsonpath "$.result.transaction.records[0].amount" == "-14367400000"
jsonpath "$.result.transaction.records[0].occupied" == 14200000000
jsonpath "$.result.transaction.records[0].block_number" == 3418132

# output #0
jsonpath "$.result.transaction.records[1].amount" == "14367200000"
jsonpath "$.result.transaction.records[1].occupied" == 14200000000
jsonpath "$.result.transaction.records[1].block_number" == 3418281

jsonpath "$.result.status" == "committed"
