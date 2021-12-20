POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_block_info",
  "params": [
    {
      "block_number": 508609
    }
  ],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.block_number" == 508609
jsonpath "$.result.block_hash" == "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70"

jsonpath "$.result.transactions" count == 1
jsonpath "$.result.transactions[0].tx_hash" == "0x32cc46179aa3d7b6eb29b9c692a9fc0b9c56d16751e42258193486d86e0fb5af"
jsonpath "$.result.transactions[0].timestamp" == 1601357943712
