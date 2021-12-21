POST {{mercury_testnet_host}}
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_transactions",
  "params": [
    {
      "item": {
          "type":"Identity",
          "value":"0x00fa22aa0aaf155a6c816634c61512046b08923111"
      },
      "asset_infos": [
        {
          "asset_type": "UDT",
          "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
        }
      ],
      "extra": null,
      "block_range": null,
      "pagination": {
        "cursor": [127, 255, 255, 255, 255, 255, 255, 254],
        "order": "desc",
        "limit": 50,
        "skip": null,
        "return_count": true
      },
      "structure_type": "DoubleEntry"
    }
  ]
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.count" == 3
jsonpath "$.result.response" count == 3

jsonpath "$.result.response[0].value.tx_hash" == "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
jsonpath "$.result.response[0].value.records" count == 16
jsonpath "$.result.response[0].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
#jsonpath "$.result.response[0].value.records[?(@.asset_info.asset_type=='UDT)]"  == 8
#jsonpath "$.result.response[0].value.records[?(@.asset_info.asset_type=='CKB')]"  == 8

jsonpath "$.result.response[1].value.tx_hash" == "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
jsonpath "$.result.response[1].value.records" count == 12
jsonpath "$.result.response[1].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
#jsonpath "$.result.response[1].value.records[?(@.asset_info.asset_type=='UDT)]"  == 6
#jsonpath "$.result.response[1].value.records[?(@.asset_info.asset_type=='CKB')]"  == 6

jsonpath "$.result.response[2].value.tx_hash" == "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
jsonpath "$.result.response[2].value.records" count == 3
jsonpath "$.result.response[2].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
