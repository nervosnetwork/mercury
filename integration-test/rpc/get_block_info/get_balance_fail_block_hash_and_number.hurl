POST {{mercury_testnet_host}}
{
  "jsonrpc": "2.0",
  "method": "get_block_info",
  "params": [
    {
      "block_hash": "0x87405a4f39154fadb13bc23cf147985208ba33d61c277ec8409722434a694e70",
      "block_number": 108710
    }
  ],
  "id": 100
}
# block_hash and block number is inconsistent.

HTTP/1.1 200
[Asserts]
jsonpath "$.error" exists
