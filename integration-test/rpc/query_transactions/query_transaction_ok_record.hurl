POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_transactions",
  "params": [
    {
      "item": {
        "type": "Record",
        "value": "3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed0000000000636b7431717136706e6777716e366539766c6d393274683834726b306c346a703268386c757263686a6d6e7776386b71337274357073663476713036793234713474633474666b677a6533356363323379707274707a66727a79677370746b7a6e"
      },
      "asset_infos": [
        {
          "asset_type": "UDT",
          "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
        }],
      "extra": null,
      "block_range": null,
      "pagination": {
        "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
        "order": "desc",
        "limit": 50,
        "skip": null,
        "return_count": true
      },
      "structure_type": "Native"
    }
  ]
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.count" == 1
jsonpath "$.result.next_cursor" exists
jsonpath "$.result.response" count == 1

jsonpath "$.result.response[0].type" == "TransactionWithRichStatus"
jsonpath "$.result.response[0].value.tx_status.status" == "committed"
jsonpath "$.result.response[0].value.tx_status.block_hash" == "0xda99cea59177843d74b7ecb11e9210de96fc580dc23155f65bf06406222f4538"
jsonpath "$.result.response[0].value.transaction.hash" == "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
#jsonpath "$.result.response[0].value.transaction.inputs[*]" count == 4
#jsonpath "$.result.response[0].value.transaction.witnesses[*]" count == 4
jsonpath "$.result.response[0].value.transaction.outputs[*]" count == 4
