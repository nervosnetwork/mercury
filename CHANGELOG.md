
# [v0.3.1](https://github.com/nervosnetwork/mercury/compare/v0.3.0...v0.3.1) (2022-03-30)

Mercury v0.3.1 optimizes the memory usage of the parallel synchronization phase, greatly reducing the memory requirements.

## 🧰 Maintenance

- fix(sync): reduce the memory usage in synchronization (#389)

# [v0.3.0](https://github.com/nervosnetwork/mercury/compare/v0.2.6...v0.3.0) (2022-03-15)

Mercury 0.3.0 has **Breaking Changes** that modifies 3 structures in the rpc and is not fully compatible with the current sdk:

- in the JsonItem type, the enum value OutPoint replaces Record
- in the Record type, the out_point field replaces the id field
- in the Balance type, renamed the freezed field to frozen

Migration instructions for this release are [here](./docs/migration.md#030-release).

## 🧰 Maintenance

- refactor(rpc): out point replaces record id （#390）
- chore(rpc): rename "freezed" to "frozen"  (#391)

## 🐛 Bug Fixes

- chore: cherry pick dev branch update (#397)

# [v0.2.6](https://github.com/nervosnetwork/mercury/compare/v0.2.5...v0.2.6) (2022-03-10)

## 🐛 Bug Fixes

- fix(rpc): Removed restriction on to address when transferring ckb (#392)

# [v0.2.5](https://github.com/nervosnetwork/mercury/compare/v0.2.4...v0.2.5) (2022-02-28)

## 🐛 Bug Fixes

- fix(sync): Mercury will fail to start when the database is newly created and the difference between node height and local db height is less than the threshold (#383)
- fix(rpc): `query_transactions` not working as expected when using `ExtraType::CellBase` and `ExtraType::Dao` input (#386)
- fix(rpc): `get_cells` and `get_cells_capacity` have invalid input filters (#385)
- fix(db): `return_count` of pagination did not work as expected (#384)
- fix(db): add pagination for `get_historical_live_cells`  (#382)

# [v0.2.4](https://github.com/nervosnetwork/mercury/compare/v0.2.3...v0.2.4) (2022-02-15)

Migration instructions for this release are [here](./docs/migration.md#024-release).

## 🚀 Features

- feat(rpc): add new transfer mode `PayWithAcp` and new rpc `get_account_info`  (#379)

## 🐛 Bug Fixes

- fix(sync): re-create table should re-create index too  (#378)
- fix(rpc): fix `get_balance` statistics for acp and pw lock cell without type script  (#377)

## 🧰 Maintenance

- chore: update rpc readme and docker config (#376)

# [v0.2.3](https://github.com/nervosnetwork/mercury/compare/v0.2.2...v0.2.3) (2022-01-19)

Mercury 0.2.3 should be paired with the latest configuration file, which has a Breaking Change: it adds the configuration for the pw lock script. Migration instructions for this release are [here](./docs/migration.md#023-release).

## 🚀 Features

- feat(rpc): support pw lock (#369)
- feat: compute progress on `ParallelSecondStage`  (#365)

## 🧰 Maintenance

- chore: update db index creation script (#372)
- feat: Add indexer API test cases (#364)
- test: add more `build_transfer_transaction` integration test cases (#367)
- feat: pprof flame graph (#368)

# [v0.2.2](https://github.com/nervosnetwork/mercury/compare/v0.2.1...v0.2.2) (2021-12-29)

In this version of Mercury, we added integration tests and fixed bugs.

## 🧰 Maintenance

- feat: add integration tests  (#352)
- perf: support pagination in tool method `get_live_cells_by_item`  (#351)
- feat: support recycling all ACP cells (#350)
- chore: update dependent lib version of rbatis and jsonrpsee (#353)

## 🐛 Bug Fixes

- fix(rpc): `query_transactions` parameter `limit` semantic error  (#354)
- fix(rpc): `query_transaction` return mistake with parameter `record`   (#357)
- fix(rpc): `get_balance` return mistake with parameter `record`  (#355)
- fix(db): `build_next_cursor` return mistake when order is desc  (#359)


# [v0.2.1](https://github.com/nervosnetwork/mercury/compare/v0.2.0...v0.2.1) (2021-12-15)

In this version of Mercury, we continue to optimize the logic of pooling money when building transactions.

## 🚀 Features

- feat(rpc): support secp udt cell to pool ckb #339
- feat(rpc): add rpc get\_sync\_state #336

## 🧰 Maintenance

- refactor(rpc): use balance method for `build_adjust_account_transaction`  #342
- refactor(rpc): update build dao with new pool method  #341
- refactor(rpc) `build_sudt_issue_transaction`  #338
- chore: add more sync log #343
- chore: Improve error info  #340

## 🐛 Bug Fixes

- fix(rpc): build transfer overflow  #334
- fix(cli): sync mode logic #337
- fix(service): overflow panic when do_sync #332
- fix(rpc): pool next at dao claim cell panic #344

# [v0.2.0](https://github.com/nervosnetwork/mercury/compare/v0.2.0-beta.4...v0.2.0) (2021-12-2)

Mercury finally ushered in the second official version. We refactored the code on a large scale and fixed some bugs. We recommend all users to use version 0.2.0, and the specific migration instructions are [here](https://github.com/nervosnetwork/mercury/blob/main/docs/migration.md).

## 🚀 Features

- feat(config): export some DB connection config mercury #323
- feat(rpc): improve transfer transaction building mercury #322
- feat(rpc): change the definition of occupied and free mercury #318
- feat(rpc): implement build_sudt_issue_transaction mercury #312

## 🐛 Bug Fixes

- fix(rpc): support repeated address registration mercury #321

## 🧰 Maintenance

- refactor(common): use full address global mercury#325
- refactor(DB): rollback tx when failed rather than close connection mercury #319
- refactor(service): use db tip to swap current tip mercury #315
- refactor(arch): split rpc module to core types and utility #301 

## 📝 Document

- docs: update README #313

# [v0.2.0-beta.4](https://github.com/nervosnetwork/mercury/compare/v0.2.0-beta.3...v0.2.0-beta.4) (2021-11-18)

Mercury adds the `indexer_cell` table in version 0.2.0-beta.4 which is a **Breaking Change**. Before upgrading, clear all data in DB and use the commands [here](https://github.com/nervosnetwork/mercury/blob/v0.2.0-beta.4/devtools/create_table/create_table.sql) to rebuild tables and indexes.

## 🚀 Features

- feat(sync): develop the synchronization of indexer cell table (#289)
- feat(rpc): forbidden negetive fee (#296) 
- feat(rpc): add timestamp for rpc query_transactions when native type (#292)
- feat(rpc): add timestamp to TransactionInfo. (#290)

## 🐛 Bug Fixes

- fix(rpc): pool_live_cells_by_items pool ckb just from first item (#305)
- fix(storage): get tx_hash when query transaction by script (#303)
- fix(rpc): pool_live_cells_by_items repeatedly pool udt for each item (#298)
- fix: always goto synchronization when restart (#283)
- fix(rpc): pool duplicate cells (#286) 
- fix(rpc): build_dao_claim_transaction panic (#282)
- fix(rpc): Fix signature_actions in build_adjust_account_transaction (#280)

## 🧰 Maintenance

- refactor(rpc): rename rpc build_smart_transfer_transaction (#288)

## 📝 Document

- docs(config): add the document of config instruction (#302)
- docs(readme): update readme (#293)


# [v0.2.0-beta.3](https://github.com/nervosnetwork/mercury/compare/v0.2.0-beta.2...v0.2.0-beta.3) (2021-11-01)

The biggest change in 0.2.0-beta.3 version is to support ckb2021 with ckb version v0.101. Besides we migrate the rust edition to 2021.

## 🚀 Features

- feat(rpc): building tx output support witnesses filling and new signature actions @EthanYuan (#264)
- feat(rpc): add new rpc build\_dao\_claim\_transaction and rename dao related rpcs @EthanYuan (#267)
- feat: add tracing proc macro and function context @KaoImin (#261)
- feat(storage): add get transaction by hashes or scripts interface @KaoImin (#256)
- feat(service): add a check of db tip and node tip @KaoImin (#275)
- feat(docker): upgrade the rust version of the build application @zhengjianhui (#269)
- feat(docker): System image upgrade for applications @zhengjianhui (#270)

## 🧰 Maintenance

- refactor: return error when sync number overflow @KaoImin (#268)
- refactor(sync): use canonical chain instead of sync status table @KaoImin (#257)
- refactor(sync): change build indexer cell table process @KaoImin (#273)
- refactor(rpc): extract build\_transaction\_with\_adjusted\_fee @EthanYuan (#255)
- refactor(rpc): change transaction status same as expression in ckb @KaoImin (#253)
- refactor(deps): upgrade ckb related dependencies to v0.101 @KaoImin (#265)
- refactor(DB): change rbatis to the official version @KaoImin (#262)
- refactor(rpc): extract calculate\_maximum\_withdraw method @EthanYuan (#252)
- refactor(SQL): add semicolons which prevent successful execution of SQL @jordanmack (#272)
- chore: upgrade rust to 1.56 and change to 2021 edition @KaoImin (#263)
- chore: remove useless monitor code @KaoImin (#277)
- chore: release v0.2.0-beta.3 version @KaoImin (#278)
- chore(docker): Mercury version fixed @zhengjianhui (#254)


# [v0.2.0-beta.2](https://github.com/nervosnetwork/mercury/compare/v0.2.0-beta.1...v0.2.0-beta.2) (2021-10-08)

## Notice

**If you are upgrading from an environment with synchronized data, execute the following [SQL command](https://github.com/nervosnetwork/mercury/blob/v0.2.0-beta.2/devtools/create_table/create_table.sql#L115:L117) to create a new table and use the latest config file before upgrading.**

## Changes

- refactor(storage): add full genesis block data and unit tests @EthanYuan (#248)
- refactor: remove rocksdb and refactor sync process @KaoImin (#247)
- refactor(rpc): remove unwrap instead of return error @KaoImin (#242)

## 🚀 Features

- feat(sync): paginate update and insert into by block number @KaoImin (#249)
- feat(cli): add some cmd args for mercury @KaoImin (#243)

## 🐛 Bug Fixes

- fix(storage): rollback block method @KaoImin (#250)
- fix(rpc): fix rpc return error @EthanYuan (#246)
- fix(sql): fix index error in live cell table @zhengjianhui (#245)
- fix(storage): update consumed info at the end of append @KaoImin (#244)



# [v0.2.0-beta.1](https://github.com/nervosnetwork/mercury/compare/v0.1.0...v0.2.0-beta.1) (2021-09-22)

Mercury released the first version of v0.2 in mid-autumn. The biggest change in v0.2 is to change the storage from RocksDB to PostgresSQL. And mercury optimizes core concepts design by using identity instead of key addresses. The config file of mercury and the JSON-RPC API has undergone great changes. The specific changes can be seen [here](https://github.com/nervosnetwork/mercury/blob/v0.2.0-beta.1/core/rpc/README.md).

## Changes

- feat: adjust mercury interface @rev-chaos (#162)
- feat(rpc): implement rpc api get\_transactions @fjchen7 (#182)
- feat(rpc): implement query\_transactions @fjchen7 (#153)
- feat(rpc): implement legacy indexer rpc api get\_live\_cells\_by\_lock\_hash @fjchen7 (#195)
- feat(rpc): implement legacy indexer rpc api get\_capacity\_by\_lock\_hash @fjchen7 (#197)
- feat(rpc): implement indexer rpc api get\_cells\_capacity @fjchen7 (#167)
- feat(rpc): impl build\_deposit\_tx @EthanYuan (#152)
- feat(rpc): add type of indexer rpc API get\_cells @fjchen7 (#161)
- feat(rpc): add build withdraw transaction rpc interface @EthanYuan (#154)
- feat(rpc): Impl udt transfer in hold by from mode @EthanYuan (#186)
- feat(rpc): Impl transfer ckb in hold by to mode @EthanYuan (#194)
- feat(rpc): Impl smart transfer @EthanYuan (#204)
- feat(rpc): Impl build transfer tx(ckb HoldByFrom and udt HoldByTo) @EthanYuan (#178)
- feat(apm): add apm tracing and derive macro @KaoImin (#148)

## 🚀 Features

- feat(sync): do synchronization in parallel @KaoImin (#151)
- feat(storage): add query historical live cell interface @KaoImin (#192)
- feat(storage): add get cell from cell table interface @KaoImin (#169)
- feat(sql): add create index sql @KaoImin (#173)
- feat(rpc): remove historical get\_balance @rev-chaos (#183)
- feat(rpc): implement indexer rpc api get\_tip @fjchen7 (#164)
- feat(rpc): check is in tx pool cache when pool cell @KaoImin (#155)
- feat(rpc): add get ckb uri interface @KaoImin (#188)
- feat(rpc): add build adjust account transaction interface @KaoImin (#170)
- feat(cli): add indexer mode option @KaoImin (#181)

## 🧰 Maintenance

- refactor: change next cursor of pagination @KaoImin (#228)
- refactor(sync): free db transaction manually @KaoImin (#175)
- refactor(sync): extend cell table and deprecate consume info table @KaoImin (#187)
- refactor(sync): change insert into mercury live cell process @KaoImin (#172)
- refactor(storage): remove uncle relationship table @KaoImin (#166)
- refactor(storage): change the  return type of standalone sql @KaoImin (#165)
- refactor(storage): change append block process @KaoImin (#159)
- refactor(rpc): swap init builtin scripts code hash @KaoImin (#177)
- refactor(rpc): remove trimmed 0x of json item @KaoImin (#176)
- refactor(rpc): multiple requests by a same connection @KaoImin (#158)
- refactor(rpc): change update adjust  account change cell @KaoImin (#201)
- refactor(common): change default order to ascend  @KaoImin (#224)

## 📝 Document

- docs: update README @KaoImin (#225)

# [v0.1.0](https://github.com/nervosnetwork/mercury/compare/v0.1.0-rc.3...v0.1.0) (2021-09-03)

## 🚀 Features

- feat(rpc): add get account number interface @KaoImin (#141)

## 🐛 Bug Fixes

- fix: get ckb locked balance and build record @KaoImin (#147)

## 🧰 Maintenance

- refactor: change the cheque key address in record @KaoImin (#143)
- refactor(rpc): change asset status @KaoImin (#142)


# [v0.1.0-rc.3](https://github.com/nervosnetwork/mercury/compare/v0.1.0-rc.2...v0.1.0-rc.3) (2021-08-01)

## 🚀 Features

- feat(rpc): filter empty balance and show details for AssetNotEnough Error @rev-chaos (#93)
- feat(service): relay ckb rpc request through mercury @KaoImin (#92)

## 📝 Document

- docs: add TOC @rev-chaos (#89)

# [v0.1.0-rc.2](https://github.com/nervosnetwork/mercury/compare/v0.1.0-rc.1...v0.1.0-rc.2) (2021-07-25)

## 🚀 Features

- feat: create acp for receiver when action is pay\_by\_from in udt transfer @rev-chaos (#41)
- feat: compatible address with acp @bitrocks (#49)
- feat: add script hash extension and register address rpc interface @KaoImin (#61)
- feat(rpc): transfer completion support using claimable source @KaoImin (#48)
- feat(rpc): set default order of query\_generic\_transaction as desc @rev-chaos (#85)
- feat(rpc): change an exact fee into fee rate when build transaction @bitrocks (#54)
- feat(rpc): build asset collection transaction @bitrocks (#67)
- feat(rpc): add query charge block interface @KaoImin (#51)
- feat(rpc): add get transaction history interface @KaoImin (#44)
- feat(rpc): add get generic transaction and generic block interface @KaoImin (#62)
- feat(rpc): add ckb rpc client that support batch request @KaoImin (#42)
- feat(extension): add index tx block number and hash extension @KaoImin (#63)

## 🐛 Bug Fixes

- fix: use filter instead of skip while in iterator @KaoImin (#46)
- fix: get others miner cellbase account iterator @KaoImin (#47)
- fix: get cell from the previous tx output cell @KaoImin (#77)
- fix: calculate tx size missing offset @bitrocks (#58)
- fix(rpc): get tx hash by script from indexer @KaoImin (#74)
- fix(rpc): cannot get cell by out point when it has been consumed @KaoImin (#65)

## 🧰 Maintenance

- refactor: storage code hash of acp address payload @KaoImin (#69)
- refactor: return error when the ckb rpc return none @KaoImin (#68)
- refactor: remove the assertation of acp script args len @KaoImin (#57)
- refactor: parse acp address from lock script @KaoImin (#86)
- refactor: get balance rpc interface and locktime extension @bitrocks (#45)
- refactor: downgrade tokio version to 0.2 @KaoImin (#50)
- refactor: change handle transfer from normal address @KaoImin (#71)
- refactor(rpc): get balance interface argument and response @KaoImin (#56)
- refactor(rpc): change the  get balance interface @KaoImin (#59)
- refactor(rpc): change handle cheque process @KaoImin (#53)
- refactor(rpc): buid transfer transaction and wallet creation @KaoImin (#64)
- refactor(architecture): split mercury to some components @KaoImin (#43)
- chore(config): change cheque config in mainet into real @KaoImin (#55)

## 📝 Document

- docs: add mercury document @rev-chaos (#80)

# [v0.1.0-rc.1](https://github.com/nervosnetwork/mercury/compare/c8ce0c522ae3aa323ee1a7edfec139c4e67a88cd...v0.1.0-rc.1) (2021-06-12)

## Features
Mercury is an indexer layer of [Nervos CKB](https://github.com/nervosnetwork/ckb). It provides powerful data indexing ability for CKB cells and scripts. Based on this, the user can easily get the balance, and even construct a transaction just like the underlying account mode by calling the JSON-RPC interface provided. More than this, mercury provides some exoteric `Action` to let users choose payment methods. See the specific function described below.
1. Get the balance of CKB or [sUDT](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0025-simple-udt/0025-simple-udt.md), which includes three kinds of balance.
2. Build a transaction that transfers CKB or sUDT to multiple identities.
3. Create a sUDT wallet.

## API
* `get_balance(udt_hash, addr)`
* `build_transfer_transaction(payload)`
* `build_wallet_creation_transaction(payload)`

For more detailed API documentation, click [here](https://github.com/nervosnetwork/ckb-sdk-java/blob/v0.42.0/ckb-mercury-sdk/README.md).