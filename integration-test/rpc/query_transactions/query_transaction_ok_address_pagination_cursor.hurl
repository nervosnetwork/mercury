# cursor is from get_transaction_ok_address_limit test cases
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
        "cursor": [0,57,125,57,0,0,0,59],
        "order": "desc",
        "limit": 2,
        "skip": null,
        "return_count": true
      },
      "structure_type": "DoubleEntry"
    }
  ]
}

HTTP/* 200
[Asserts]
jsonpath "$.result.count" == 2
jsonpath "$.result.response" count == 2
jsonpath "$.result.response[*].type" includes "TransactionInfo"

#cursor: jsonpath "$.result.next_cursor" == null
