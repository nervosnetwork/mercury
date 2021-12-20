POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "item": {
        "type": "Address",
        "value": "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
      },
      "asset_infos": [
        {
          "asset_type": "UDT",
          "udt_hash": "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"
        }
      ],
      "tip_block_number": 3767610
    }
  ],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.balances" count == 1

jsonpath "$.result.balances[0].ownership.type" == "Address"
jsonpath "$.result.balances[0].ownership.value" == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"

jsonpath "$.result.balances[0].asset_info.asset_type" == "UDT"
jsonpath "$.result.balances[0].asset_info.udt_hash" == "0xf21e7350fa9518ed3cbb008e0e8c941d7e01a12181931d5608aa366ee22228bd"

jsonpath "$.result.balances[0].free" == "60"
jsonpath "$.result.balances[0].occupied" == "0"
jsonpath "$.result.balances[0].freezed" == "0"
jsonpath "$.result.balances[0].claimable" == "0"

jsonpath "$.result.tip_block_number" == 3767610
