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
    "structure_type": "DoubleEntry"
  }]
}

# The returned tx: https://explorer.nervos.org/aggron/transaction/0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1
HTTP/1.1 200
[Asserts]
jsonpath "$.result.type" == "TransactionInfo"
jsonpath "$.result.value" exists
jsonpath "$.result.value.tx_hash" == "0x3020e90284f2d4f51b79471245939d043e958c60ba5dc95b212f8cbd8b875bd1"

jsonpath "$.result.value.records[*]" count == 8

# input #1
jsonpath "$.result.value.records[?(@.amount=='-561599924694')].ownership.value" == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdrhpvcu82numz73852ed45cdxn4kcn72cr4338a"

# output #0
jsonpath "$.result.value.records[?(@.amount=='100')].ownership.value" == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqv8c706nuqkxchp0sw996qqunhmkkgesqs0rppmc"

# output #2
jsonpath "$.result.value.records[?(@.amount=='545399923736')].ownership.value" == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdrhpvcu82numz73852ed45cdxn4kcn72cr4338a"
