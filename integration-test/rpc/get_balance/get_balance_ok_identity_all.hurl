POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "item": {
          "type":"Identity",
          "value":"0x00fa22aa0aaf155a6c816634c61512046b08923111"
      },
      "asset_infos": [],
      "tip_block_number": 3769130
    }
  ],
  "id": 100
}

HTTP/1.1 200
[Asserts]
jsonpath "$.result.balances" count == 3

jsonpath "$.result.balances[?(@.free=='60')].asset_info.asset_type" == "UDT"
jsonpath "$.result.balances[?(@.free=='60')].ownership.value" == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
jsonpath "$.result.balances[?(@.free=='1000000000000')].asset_info.asset_type" == "CKB"
jsonpath "$.result.balances[?(@.free=='1000000000000')].ownership.value" == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9"
jsonpath "$.result.balances[?(@.free=='1979699999470')].asset_info.asset_type" == "CKB"
jsonpath "$.result.balances[?(@.free=='1979699999470')].ownership.value" == "ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vq06y24q4tc4tfkgze35cc23yprtpzfrzygsptkzn"
