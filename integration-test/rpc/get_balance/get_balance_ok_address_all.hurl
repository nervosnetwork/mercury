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
      "asset_infos": [],
      "tip_block_number": 3767610
    }
  ],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.balances" count == 2

jsonpath "$.result.balances[?(@.free=='60')].asset_info.asset_type" == "UDT"
jsonpath "$.result.balances[?(@.free=='60')].ownership.value" == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"

jsonpath "$.result.balances[?(@.free=='985799999470')].asset_info.asset_type" == "CKB"
jsonpath "$.result.balances[?(@.free=='985799999470')].ownership.value" == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
