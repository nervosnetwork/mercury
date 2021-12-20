POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "get_spent_transaction",
  "params": [{
    "outpoint": {
      "tx_hash": "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112",
      "index": "0x2"
    },
    "structure_type": "Native"
  }]
}

# The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
HTTP/1.1 200
[Asserts]
jsonpath "$.result.type" == "TransactionWithRichStatus"
jsonpath "$.result.value" exists
jsonpath "$.result.value.transaction" exists
jsonpath "$.result.value.transaction.hash" == "0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1"

jsonpath "$.result.value.transaction.inputs" count == 2
jsonpath "$.result.value.transaction.inputs[1].previous_output.tx_hash" == "0xa4aee7ae950d7fb74271816566827832d50ebf3fc04234927314fd332c4cd112"
jsonpath "$.result.value.transaction.inputs[1].previous_output.index" == "0x2"

jsonpath "$.result.value.transaction.outputs" count == 3
jsonpath "$.result.value.transaction.outputs[?(@.capacity=='0x7efc5e6418')].lock.args" == "0xa3b8598e1d53e6c5e89e8acb6b4c34d3adb13f2b"
jsonpath "$.result.value.transaction.outputs[?(@.capacity=='0x7efc5e6418')].type" == null
jsonpath "$.result.value.transaction.outputs[?(@.capacity=='0x34e62ce00')].lock.args" == "0xa3b8598e1d53e6c5e89e8acb6b4c34d3adb13f2b"


jsonpath "$.result.value.transaction.outputs_data" count == 3
jsonpath "$.result.value.transaction.witnesses" count == 2

jsonpath "$.result.value.tx_status.status" == "committed"
jsonpath "$.result.value.tx_status.block_hash" == "0x54af022e0fdebed3690ee6d6e2368f3c976c6ad6cfcdbfa307869fd57c2d5129"
