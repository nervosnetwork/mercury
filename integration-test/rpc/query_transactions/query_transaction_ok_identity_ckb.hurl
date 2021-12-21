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
          "asset_type": "CKB",
          "udt_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
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
jsonpath "$.result.count" == 1
jsonpath "$.result.response" count == 1

jsonpath "$.result.response[0].type" == "TransactionInfo"
jsonpath "$.result.response[0].value.tx_hash" == "0x9bd14d72c04087e6aa1caac4531cff12853017003b24236b2382aef92410f996"
jsonpath "$.result.response[0].value.records" count == 5
jsonpath "$.result.response[0].value.records[*].ownership.value" includes "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqflz4emgssc6nqj4yv3nfv2sca7g9dzhscgmg28x"
jsonpath "$.result.response[0].value.records[*].ownership.value" includes "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
