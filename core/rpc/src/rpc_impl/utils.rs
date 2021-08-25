use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER,
    DAO_CODE_HASH, SECP256K1_CODE_HASH, SUDT_CODE_HASH,
};
use crate::types::{
    decode_record_id, encode_record_id, AssetType, DaoState, ExtraFilter, IOType, Identity,
    IdentityFlag, Item, Record, Status,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, parse_address};
use common::{
    Address, AddressPayload, DetailedCell, Order, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, SECP256K1,
};
use core_storage::DBAdapter;

use ckb_types::core::{BlockNumber, RationalU256, TransactionView};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::convert::TryInto;
use std::str::FromStr;

impl<C: CkbRpc + DBAdapter> MercuryRpcImpl<C> {
    pub(crate) fn get_script_builder(&self, script_name: &str) -> packed::ScriptBuilder {
        self.builtin_scripts
            .get(script_name)
            .cloned()
            .unwrap()
            .script
            .as_builder()
    }

    #[allow(clippy::unnecessary_unwrap)]
    pub(crate) fn get_scripts_by_identity(
        &self,
        ident: Identity,
        lock_filter: Option<H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut scripts = Vec::new();

        let (flag, pubkey_hash) = ident.parse();
        match flag {
            IdentityFlag::Ckb => {
                if lock_filter.is_none()
                    || lock_filter.clone().unwrap() == **SECP256K1_CODE_HASH.load()
                {
                    // get secp script
                    let secp_script = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    scripts.push(secp_script);
                } else if lock_filter.is_none()
                    || lock_filter.clone().unwrap() == **ACP_CODE_HASH.load()
                {
                    let acp_script_20 = self
                        .get_script_builder(ACP)
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    scripts.push(acp_script_20);
                } else if lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load()
                {
                    let secp_script = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    let _lock_hash: H256 = secp_script.calc_script_hash().unpack();
                    // let cheque_scripts = self.db.get_scripts(cheque_code_hash, lock_hash..)?;
                    // scripts.append(cheque_scripts);
                }
            }
            _ => {
                todo!();
            }
        }

        Ok(scripts)
    }

    #[allow(clippy::unnecessary_unwrap, clippy::collapsible_if)]
    // TODO@gym: refactor here.
    pub(crate) fn get_scripts_by_address(
        &self,
        addr: &Address,
        lock_filter: Option<H256>,
    ) -> InnerResult<Vec<packed::Script>> {
        let mut ret = Vec::new();
        let script = address_to_script(addr.payload());

        if lock_filter.is_none() || lock_filter.clone().unwrap() == **SECP256K1_CODE_HASH.load() {
            if self.is_script(&script, SECP256K1) {
                ret.push(script);
            }
        } else if lock_filter.is_none() || lock_filter.clone().unwrap() == **ACP_CODE_HASH.load() {
            if self.is_script(&script, ACP) {
                ret.push(script);
            }
        } else if lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load() {
            if self.is_script(&script, CHEQUE) {
                let _lock_hash: H256 = script.calc_script_hash().unpack();
                // let cheque_scripts = self.db.get_scripts(cheque_code_hash, lock_hash..)?;
                // scripts.append(cheque_scripts);
            }
        }

        Ok(ret)
    }

    pub(crate) async fn get_live_cells_by_item(
        &self,
        item: Item,
        asset_type: AssetType,
        _tip_number: BlockNumber,
        lock_filter: Option<H256>,
        extra: Option<ExtraFilter>,
        range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<Vec<DetailedCell>> {
        let type_hashes = match asset_type {
            AssetType::Ckb => match extra {
                Some(ExtraFilter::Dao(_)) => vec![self
                    .builtin_scripts
                    .get(DAO)
                    .cloned()
                    .unwrap()
                    .script
                    .calc_script_hash()
                    .unpack()],
                _ => vec![],
            },
            AssetType::UDT(udt_hash) => vec![udt_hash],
        };

        let ret = match item {
            Item::Identity(ident) => {
                let scripts = self.get_scripts_by_identity(ident.clone(), lock_filter)?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                let cells = self
                    .storage
                    .get_live_cells(None, lock_hashes, type_hashes, None, range, pagination)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                let (_flag, pubkey_hash) = ident.parse();
                let secp_lock_hash: H256 = self
                    .get_script_builder(SECP256K1)
                    .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                    .build()
                    .calc_script_hash()
                    .unpack();

                cells
                    .response
                    .into_iter()
                    .filter(|cell| self.filter_useless_cheque(cell, &secp_lock_hash))
                    .collect()
            }

            Item::Address(addr) => {
                let addr = Address::from_str(&addr).unwrap();
                let scripts = self.get_scripts_by_address(&addr, lock_filter)?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                let cells = self
                    .storage
                    .get_live_cells(None, lock_hashes, type_hashes, None, range, pagination)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                cells
                    .response
                    .into_iter()
                    .filter(|cell| {
                        self.filter_useless_cheque(
                            &cell,
                            &address_to_script(addr.payload())
                                .calc_script_hash()
                                .unpack(),
                        )
                    })
                    .collect()
            }

            Item::Record(id) => {
                let mut cells = vec![];
                let (out_point, address) = decode_record_id(id);
                let mut lock_hashes = vec![];
                if lock_filter.is_some() {
                    lock_hashes.push(lock_filter.unwrap());
                }

                let cell = self
                    .storage
                    .get_live_cells(
                        Some(out_point),
                        lock_hashes,
                        type_hashes,
                        None,
                        range,
                        pagination,
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                let cell = cell.response.get(0).cloned().unwrap();
                let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

                if code_hash == **CHEQUE_CODE_HASH.load() {
                    let secp_lock_hash: H256 = if address.is_secp256k1() {
                        address_to_script(address.payload())
                            .calc_script_hash()
                            .unpack()
                    } else {
                        todo!()
                    };

                    let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();
                    let is_useful = if self
                        .is_cheque_timeout(RationalU256::from_u256(cell.epoch_number.clone()), None)
                    {
                        cell_args[20..40] == secp_lock_hash.0[0..20]
                    } else {
                        cell_args[0..20] == secp_lock_hash.0[0..20]
                    };

                    if is_useful {
                        cells.push(cell);
                    }
                } else {
                    cells.push(cell);
                }

                cells
            }
        };

        if extra == Some(ExtraFilter::CellBase) {
            Ok(ret.into_iter().filter(|cell| cell.tx_index == 0).collect())
        } else {
            Ok(ret)
        }
    }

    pub(crate) async fn get_transactions_by_item(
        &self,
        item: Item,
        asset_type: AssetType,
        extra: Option<ExtraFilter>,
        range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<Vec<TransactionView>> {
        let type_hashes = match asset_type {
            AssetType::Ckb => match extra {
                Some(ExtraFilter::Dao(_)) => vec![self
                    .builtin_scripts
                    .get(DAO)
                    .cloned()
                    .unwrap()
                    .script
                    .calc_script_hash()
                    .unpack()],
                _ => vec![],
            },
            AssetType::UDT(udt_hash) => vec![udt_hash],
        };

        let ret = match item {
            Item::Identity(ident) => {
                let scripts = self.get_scripts_by_identity(ident, None)?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                self.storage
                    .get_transactions(vec![], lock_hashes, type_hashes, range, pagination)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            }

            Item::Address(addr) => {
                let addr = parse_address(&addr)
                    .map_err(|e| RpcErrorMessage::CommonError(e.to_string()))?;
                let scripts = self.get_scripts_by_address(&addr, None)?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<_>>();
                self.storage
                    .get_transactions(vec![], lock_hashes, type_hashes, range, pagination)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            }

            Item::Record(id) => {
                let (outpoint, _addr) = decode_record_id(id);
                self.storage
                    .get_transactions(
                        vec![outpoint.tx_hash().unpack()],
                        vec![],
                        type_hashes,
                        range,
                        pagination,
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            }
        };

        if extra == Some(ExtraFilter::CellBase) {
            Ok(ret
                .response
                .into_iter()
                .filter(|tx| tx.is_cellbase())
                .collect())
        } else {
            Ok(ret.response)
        }
    }

    pub(crate) fn pool_asset(
        pool_cells: &mut Vec<DetailedCell>,
        amount_required: &mut BigInt,
        resource_cells: Vec<DetailedCell>,
        is_ckb: bool,
    ) -> bool {
        let zero = BigInt::from(0);
        for cell in resource_cells.iter() {
            if *amount_required <= zero {
                return true;
            }

            let amount = if is_ckb {
                let capacity: u64 = cell.cell_output.capacity().unpack();
                capacity as u128
            } else {
                decode_udt_amount(&cell.cell_data)
            };

            *amount_required -= amount;
            pool_cells.push(cell.clone());
        }

        *amount_required <= zero
    }

    pub(crate) fn get_secp_lock_hash_by_item(&self, item: Item) -> InnerResult<H160> {
        match item {
            Item::Identity(ident) => {
                let (flag, pubkey_hash) = ident.parse();
                match flag {
                    IdentityFlag::Ckb => {
                        let lock_hash: H256 = self
                            .get_script_builder(SECP256K1)
                            .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                            .build()
                            .calc_script_hash()
                            .unpack();
                        Ok(H160::from_slice(&lock_hash.0[0..20]).unwrap())
                    }
                    _ => unreachable!(),
                }
            }

            Item::Address(addr) => {
                let addr = parse_address(&addr)
                    .map_err(|e| RpcErrorMessage::CommonError(e.to_string()))?;
                let script = address_to_script(&addr.payload());
                if self.is_script(&script, SECP256K1) || self.is_script(&script, ACP) {
                    Ok(H160::from_slice(&script.args().raw_data()[0..20]).unwrap())
                } else {
                    unreachable!();
                }
            }

            Item::Record(id) => {
                let (_, address) = decode_record_id(id);
                self.get_secp_lock_hash_by_item(Item::Address(address.to_string()))
            }
        }
    }

    pub(crate) async fn to_record(
        &self,
        cell: &DetailedCell,
        io_type: IOType,
    ) -> InnerResult<Vec<Record>> {
        let mut records = vec![];

        let udt_record = if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == **SUDT_CODE_HASH.load() {
                let address = self.generate_udt_address(cell, &io_type).await?;
                let id = encode_record_id(cell.out_point.clone(), address.clone());
                let asset_type = AssetType::UDT(type_script.calc_script_hash().unpack());
                let status = self.generate_udt_status(cell, &io_type).await?;
                let amount = self.generate_udt_amount(cell, &io_type);
                let extra = None;

                Some(Record {
                    id,
                    address: address.to_string(),
                    asset_type,
                    amount: amount.try_into().unwrap(),
                    status,
                    extra,
                })
            } else {
                None
            }
        } else {
            None
        };

        if udt_record.is_some() {
            records.push(udt_record.unwrap());
        }

        let address = self.generate_ckb_address(cell)?;
        let id = encode_record_id(cell.out_point.clone(), address.clone());
        let asset_type = AssetType::Ckb;
        let status = self.generate_ckb_status(cell, &io_type);
        let amount = self.generate_ckb_amount(cell, &io_type);
        let extra = self.generate_extra(cell, io_type).await?;
        let ckb_record = Record {
            id,
            address: address.to_string(),
            asset_type,
            amount: amount.try_into().unwrap(),
            status,
            extra,
        };
        records.push(ckb_record);

        Ok(records)
    }

    fn generate_ckb_address(&self, cell: &DetailedCell) -> InnerResult<Address> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(self.script_to_address(&cell.cell_output.lock()));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            todo!();
        }

        Ok(self.script_to_address(&cell.cell_output.lock()))
    }

    fn generate_ckb_status(&self, cell: &DetailedCell, io_type: &IOType) -> Status {
        let block_num = if io_type == &IOType::Input {
            0
        } else {
            cell.block_number
        };

        Status::Fixed(block_num)
    }

    fn generate_ckb_amount(&self, cell: &DetailedCell, io_type: &IOType) -> BigInt {
        let capacity: u64 = cell.cell_output.capacity().unpack();
        match io_type {
            IOType::Input => BigInt::from(capacity) * -1,
            IOType::Output => BigInt::from(capacity),
        }
    }

    async fn generate_udt_address(
        &self,
        cell: &DetailedCell,
        io_type: &IOType,
    ) -> InnerResult<Address> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(self.script_to_address(&cell.cell_output.lock()));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let (mut generate_epoch_num, mut judge_epoch_num) =
                (RationalU256::zero(), RationalU256::zero());

            if io_type == &IOType::Input {
                generate_epoch_num = self
                    .storage
                    .get_transaction_info_by_hash(cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .epoch_number;
                judge_epoch_num = RationalU256::from_u256(cell.epoch_number.clone());
            } else {

                // let transaction = self.db.get_spent_transaction(cell.out_point);
                // generate_epoch_num = cell.epoch_number;
                // judge_epoch_num = if transaction.is_some() {
                //     transaction.unwrap().epoch_num
                // } else {
                //     tip_epoch
                // };
            }

            let lock_hash_160 = if self.is_cheque_timeout(generate_epoch_num, Some(judge_epoch_num))
            {
                cell.cell_output.lock().args().raw_data()[20..40].to_vec()
            } else {
                cell.cell_output.lock().args().raw_data()[0..20].to_vec()
            };

            let res = self
                .storage
                .get_scripts(
                    vec![H160::from_slice(&lock_hash_160).unwrap()],
                    vec![],
                    None,
                    vec![],
                    PaginationRequest::default().set_limit(Some(1)),
                )
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if res.response.is_empty() {
                return Err(RpcErrorMessage::CannotGetScriptByHash);
            } else {
                return Ok(self.script_to_address(res.response.get(0).unwrap()));
            }
        }

        Ok(self.script_to_address(&cell.cell_output.lock()))
    }

    fn generate_udt_amount(&self, cell: &DetailedCell, io_type: &IOType) -> BigInt {
        let amount = BigInt::from(decode_udt_amount(&cell.cell_data));
        match io_type {
            IOType::Input => amount * -1,
            IOType::Output => amount,
        }
    }

    async fn generate_udt_status(
        &self,
        cell: &DetailedCell,
        io_type: &IOType,
    ) -> InnerResult<Status> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            let block_number = if io_type == &IOType::Input {
                self.storage
                    .get_transaction_info_by_hash(cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .block_number
            } else {
                cell.block_number
            };

            return Ok(Status::Fixed(block_number));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            // let transaction = self.storage.get_spent_transaction(cell.out_point);
            //     if transaction.is_some() {
            //         return Status::Fixed(transaction.block_num);
            //     } else {
            //         if cheque_timeout(cell.epoch_num, tip_epoch) {
            //             let timeout_block_num = block_num + ...;
            //             return Status::Fixed(timeout_block_num);
            //         } else {
            //             return Status::Claimable(block_num);
            //         }
            //     }
        }

        todo!()
    }

    async fn generate_extra(
        &self,
        cell: &DetailedCell,
        io_type: IOType,
    ) -> InnerResult<Option<ExtraFilter>> {
        if cell.tx_index == 0 && io_type == IOType::Output {
            return Ok(Some(ExtraFilter::CellBase));
        }

        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let _type_code_hash: H256 = type_script.code_hash().unpack();

            // if type_code_hash == **DAO_CODE_HASH.load() {
            //     let block_num = if io_type == IOType::Input {
            //        self.storage.get_block_number_by_transaction(cell.out_point.tx_hash().unpack()).await.map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
            //    } else {
            //        cell.block_number
            //    };

            //     let (status, start_num, end_num) = if cell.cell_data == Bytes::from(vec![0,0,0,0]) {
            //        let tip_hash = self.db.get_canonical_block_hash(tip_number);
            //        (DaoState::Deposit(block_num), cell.block_hash, tip_hash)
            //     } else {
            //         let deposit_block_num = parse(cell.data);
            //         let start_hash = self.db.get_canonical_block_hash(deposit_block_num);
            //         (DaoState::Withdraw(block_num), start_hash, cell.block_hash)
            //     }

            //     let start_AR = self.storage.get_block_header(Some(start_hash), None).await.map_err(|e| RpcErrorMessage::DBError(e.to_string()))?.dao.AR;
            //     let end_AR = self.db.get_header(end_hash).dao.AR;

            //     let reward = cell.capacity * end_AR / start_AR - cell.capacity;
            //     Some(ExtraFilter::)
            // }
        }
        Ok(None)
    }

    fn filter_useless_cheque(&self, cell: &DetailedCell, secp_lock_hash: &H256) -> bool {
        let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if code_hash == **CHEQUE_CODE_HASH.load() {
            let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();

            if self.is_cheque_timeout(RationalU256::from_u256(cell.epoch_number.clone()), None) {
                cell_args[20..40] == secp_lock_hash.0[0..20]
            } else {
                cell_args[0..20] == secp_lock_hash.0[0..20]
            }
        } else {
            true
        }
    }

    pub(crate) fn is_script(&self, script: &packed::Script, script_name: &str) -> bool {
        let s = self
            .builtin_scripts
            .get(script_name)
            .cloned()
            .unwrap()
            .script;
        script.code_hash() == s.code_hash() && script.hash_type() == s.hash_type()
    }

    pub(crate) fn is_cheque_timeout(&self, from: RationalU256, end: Option<RationalU256>) -> bool {
        if let Some(cur_epoch) = end {
            cur_epoch - from > self.cheque_since
        } else {
            &*CURRENT_EPOCH_NUMBER.load().clone() - from > self.cheque_since
        }
    }

    pub(crate) fn script_to_address(&self, script: &packed::Script) -> Address {
        let payload = AddressPayload::from_script(script, self.network_type);
        Address::new(self.network_type, payload)
    }
}
