POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": [
    {
      "item": {
        "type": "Record",
        "value": "ea0b230104fd3be2cc33ab50c3d591dc6cefbe8ed83f7e63c8142de4b5a0ee720000000000636b7431717a646130637230386d38356863386a6c6e6670337a65723778756c656a79777434396b7432727230767468797761613530787773717736766a7a79396b616878336c79766c67617038647038657764386738307063676365787a726a"
      },
      "asset_infos": []
    }
  ],
  "id": 100
}
# live cell of the given record: output #0 in tx 0xea0b230104fd3be2cc33ab50c3d591dc6cefbe8ed83f7e63c8142de4b5a0ee72

HTTP/1.1 200
[Asserts]
jsonpath "$.result.balances" count == 1

jsonpath "$.result.balances[0].ownership.type" == "Address"
jsonpath "$.result.balances[0].ownership.value" == "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqw6vjzy9kahx3lyvlgap8dp8ewd8g80pcgcexzrj"
jsonpath "$.result.balances[0].asset_info.asset_type" == "CKB"
jsonpath "$.result.balances[0].free" == "194703317445"
