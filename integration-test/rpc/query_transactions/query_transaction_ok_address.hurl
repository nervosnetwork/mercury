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
jsonpath "$.result.count" == 4
jsonpath "$.result.next_cursor" == null
jsonpath "$.result.response" count == 4
jsonpath "$.result.response[*].type" includes "TransactionInfo"

jsonpath "$.result.response[0].value.tx_hash" == "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996"
jsonpath "$.result.response[0].value.records" count == 5
jsonpath "$.result.response[0].value.records[*].ownership.value" includes "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqflz4emgssc6nqj4yv3nfv2sca7g9dzhscgmg28x"
jsonpath "$.result.response[0].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"

jsonpath "$.result.response[1].value.tx_hash" == "0x3eb0a1974dd6a2b6c3ba220169cef6eec21e94d2267fab9a4e810accc693c8ed"
jsonpath "$.result.response[1].value.records" count == 16
jsonpath "$.result.response[1].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
#jsonpath "$.result.response[1].value.records[?(@.asset_info.asset_type=='UDT)]"  == 8
#jsonpath "$.result.response[1].value.records[?(@.asset_info.asset_type=='CKB')]"  == 8

jsonpath "$.result.response[2].value.tx_hash" == "0x5b0b303647d191677e53b6d94bbeda36794ca6599705b4b4b7f693409bb915e3"
jsonpath "$.result.response[2].value.records" count == 12
jsonpath "$.result.response[2].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
#jsonpath "$.result.response[2].value.records[?(@.asset_info.asset_type=='UDT)]"  == 6
#jsonpath "$.result.response[2].value.records[?(@.asset_info.asset_type=='CKB')]"  == 6

jsonpath "$.result.response[3].value.tx_hash" == "0xc095eefa53e137e6e7be70b1df836513e5b28a4578845f7aa26853d456a9887f"
jsonpath "$.result.response[3].value.records" count == 3
jsonpath "$.result.response[3].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
