POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_transactions",
  "params": [
    {
      "item": {
        "type": "Address",
        "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
      },
      "asset_infos": [],
      "extra": null,
      "block_range": null,
      "pagination": {
        "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
        "order": "desc",
        "limit": 6,
        "skip": null,
        "return_count": true
      },
      "structure_type": "Native"
    }
  ]
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.count" == 4
jsonpath "$.result.next_cursor" == null
jsonpath "$.result.response" count == 4

jsonpath "$.result.response[0].type" == "TransactionWithRichStatus"
jsonpath "$.result.response[0].value.tx_status.status" == "committed"
jsonpath "$.result.response[0].value.tx_status.block_hash" == "0xb12c682135282fe03b2d5c2c1f11f3add67c8f88e1b1914dfa1fc67c30dbb107"
jsonpath "$.result.response[0].value.transaction.hash" == "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996"
#jsonpath "$.result.response[0].value.transaction.inputs[*]" count == 1
#jsonpath "$.result.response[0].value.transaction.witnesses[*]" count == 1
jsonpath "$.result.response[0].value.transaction.outputs[*]" count == 4

jsonpath "$.result.response[1].type" == "TransactionWithRichStatus"
jsonpath "$.result.response[1].value.tx_status.status" == "committed"
jsonpath "$.result.response[1].value.tx_status.block_hash" == "0xda99cea59177843d74b7ecb11e9210de96fc580dc23155f65bf06406222f4538"
jsonpath "$.result.response[1].value.transaction.hash" == "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
#jsonpath "$.result.response[1].value.transaction.inputs[*]" count == 4
#jsonpath "$.result.response[1].value.transaction.witnesses[*]" count == 4
jsonpath "$.result.response[1].value.transaction.outputs[*]" count == 4
