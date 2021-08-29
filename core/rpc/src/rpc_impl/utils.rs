use crate::error::{InnerResult, RpcErrorMessage};
use crate::rpc_impl::{
    address_to_script, ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER,
    DAO_CODE_HASH, SECP256K1_CODE_HASH, SUDT_CODE_HASH,
};
use crate::types::{
    decode_record_id, encode_record_id, AddressOrLockHash, AssetInfo, AssetType, Balance, DaoInfo, DaoState,
    ExtraFilter, IOType, Identity, IdentityFlag, Item, Record, RequiredUDT, SignatureEntry, Source, Status,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::{decode_dao_block_number, decode_udt_amount, parse_address};
use common::{
    Address, AddressPayload, DetailedCell, Order, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, SECP256K1,
};
use core_storage::DBAdapter;

use ckb_types::core::{
    BlockNumber, Capacity, EpochNumberWithFraction, RationalU256, TransactionView,
};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::{HashMap, HashSet};
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
    pub(crate) async fn get_scripts_by_identity(
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
                    let mut acp_scripts = self
                        .storage
                        .get_script_by_partical_arg(
                            (**ACP_CODE_HASH.load()).clone(),
                            Bytes::from(pubkey_hash.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                    scripts.append(&mut acp_scripts);
                } else if lock_filter.is_none() || lock_filter.unwrap() == **CHEQUE_CODE_HASH.load()
                {
                    let secp_script = self
                        .get_script_builder(SECP256K1)
                        .args(Bytes::from(pubkey_hash.0.to_vec()).pack())
                        .build();
                    let lock_hash: H256 = secp_script.calc_script_hash().unpack();
                    let lock_hash_160 = H160::from_slice(&lock_hash.0[0..20]).unwrap();

                    let mut receiver_cheque = self
                        .storage
                        .get_script_by_partical_arg(
                            (**CHEQUE_CODE_HASH.load()).clone(),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (0, 20),
                        )
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                    let mut sender_cheque = self
                        .storage
                        .get_script_by_partical_arg(
                            (**CHEQUE_CODE_HASH.load()).clone(),
                            Bytes::from(lock_hash_160.0.to_vec()),
                            (20, 40),
                        )
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                    scripts.append(&mut receiver_cheque);
                    scripts.append(&mut sender_cheque);
                }
            }
            _ => {
                unreachable!();
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
        asset_infos: HashSet<AssetInfo>,
        tip_block_number: Option<BlockNumber>,
        tip_epoch_number: Option<RationalU256>,
        lock_filter: Option<H256>,
        extra: Option<ExtraFilter>,
    ) -> InnerResult<Vec<DetailedCell>> {
        let type_hashes = asset_infos
            .into_iter()
            .map(|asset_info| match asset_info.asset_type {
                AssetType::Ckb => match extra {
                    Some(ExtraFilter::Dao(_)) => self
                        .builtin_scripts
                        .get(DAO)
                        .cloned()
                        .unwrap()
                        .script
                        .calc_script_hash()
                        .unpack(),
                    _ => H256::default(),
                },
                AssetType::UDT => asset_info.udt_hash,
            })
            .collect();

        let ret = match item {
            Item::Identity(ident) => {
                let scripts = self
                    .get_scripts_by_identity(ident.clone(), lock_filter)
                    .await?;
                let lock_hashes = scripts
                    .iter()
                    .map(|script| script.calc_script_hash().unpack())
                    .collect::<Vec<H256>>();
                let cells = self
                    .storage
                    .get_live_cells(
                        None,
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        PaginationRequest::default(),
                    )
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
                    .filter(|cell| {
                        self.filter_useless_cheque(cell, &secp_lock_hash, tip_epoch_number.clone())
                    })
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
                    .get_live_cells(
                        None,
                        lock_hashes,
                        type_hashes,
                        tip_block_number,
                        None,
                        PaginationRequest::default(),
                    )
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
                            tip_epoch_number.clone(),
                        )
                    })
                    .collect()
            }

            Item::Record(id) => {
                let mut cells = vec![];
                let (out_point, address_or_lock_hash) = decode_record_id(id)?;
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
                        tip_block_number,
                        None,
                        PaginationRequest::default(),
                    )
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;

                let cell = cell.response.get(0).cloned().unwrap();
                let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

                if code_hash == **CHEQUE_CODE_HASH.load() {
                    let secp_lock_hash: H160 = match address_or_lock_hash {
                        AddressOrLockHash::Address(address) => {
                            let address = parse_address(&address)
                                .map_err(|e| RpcErrorMessage::CommonError(e.to_string()))?;

                            let lock_hash: H256 = address_to_script(address.payload())
                                .calc_script_hash()
                                .unpack();
                            H160::from_slice(&lock_hash.0[0..20]).unwrap()
                        }
                        AddressOrLockHash::LockHash(lock_hash) => {
                            H160::from_str(&lock_hash).unwrap()
                        }
                    };

                    let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();
                    let is_useful = if self.is_unlock(
                        RationalU256::from_u256(cell.epoch_number.clone()),
                        tip_epoch_number.clone(),
                        self.cheque_timeout.clone(),
                    ) {
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
        asset_info: AssetInfo,
        extra: Option<ExtraFilter>,
        range: Option<Range>,
        pagination: PaginationRequest,
    ) -> InnerResult<Vec<TransactionView>> {
        let type_hashes = match asset_info.asset_type {
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
            AssetType::UDT => vec![asset_info.udt_hash.clone()],
        };

        let ret = match item {
            Item::Identity(ident) => {
                let scripts = self.get_scripts_by_identity(ident, None).await?;
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
                let (outpoint, _address_or_lock_hash) = decode_record_id(id)?;
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
        &self,
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
                let (_, address_or_lock_hash) = decode_record_id(id)?;
                match address_or_lock_hash {
                    AddressOrLockHash::Address(address) => {
                        Ok(self.get_secp_lock_hash_by_item(Item::Address(address))?)
                    }
                    AddressOrLockHash::LockHash(lock_hash) => {
                        Ok(H160::from_str(&lock_hash).unwrap())
                    }
                }
            }
        }
    }

    pub(crate) async fn to_record(
        &self,
        cell: &DetailedCell,
        io_type: IOType,
        tip_block_number: BlockNumber,
        tip_epoch_number: RationalU256,
    ) -> InnerResult<Vec<Record>> {
        let mut records = vec![];

        let udt_record = if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == **SUDT_CODE_HASH.load() {
                let address_or_lock_hash = self
                    .generate_udt_address_or_lock_hash(cell, &io_type, &tip_epoch_number)
                    .await?;
                let id = encode_record_id(cell.out_point.clone(), address_or_lock_hash.clone());
                let asset_info = AssetInfo::new_udt(type_script.calc_script_hash().unpack());
                let status = self
                    .generate_udt_status(cell, &io_type, &tip_epoch_number)
                    .await?;
                let amount = self.generate_udt_amount(cell, &io_type);
                let extra = None;

                Some(Record {
                    id,
                    address_or_lock_hash,
                    asset_info,
                    amount: amount.to_string(),
                    occupied: 0,
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

        let address_or_lock_hash = self.generate_ckb_address_or_lock_hash(cell).await?;
        let id = encode_record_id(cell.out_point.clone(), address_or_lock_hash.clone());
        let asset_info = AssetInfo::new_ckb();
        let status = self.generate_ckb_status(cell, &io_type);
        let amount = self.generate_ckb_amount(cell, &io_type);
        let extra = self.generate_extra(cell, io_type, tip_block_number).await?;
        let data_occupied = Capacity::bytes(cell.cell_data.len())
            .map_err(|e| RpcErrorMessage::OccupiedCapacityError(e.to_string()))?;
        let occupied = cell
            .cell_output
            .occupied_capacity(data_occupied)
            .map_err(|e| RpcErrorMessage::OccupiedCapacityError(e.to_string()))?;
        let ckb_record = Record {
            id,
            address_or_lock_hash,
            asset_info,
            amount: amount.to_string(),
            occupied: occupied.as_u64(),
            status,
            extra,
        };
        records.push(ckb_record);

        Ok(records)
    }

    async fn generate_ckb_address_or_lock_hash(
        &self,
        cell: &DetailedCell,
    ) -> InnerResult<AddressOrLockHash> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(AddressOrLockHash::Address(
                self.script_to_address(&cell.cell_output.lock()).to_string(),
            ));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let sender_lock_hash_160 = cell.cell_output.lock().args().raw_data()[20..40].to_vec();
            let lock_hash = H160::from_slice(&sender_lock_hash_160).unwrap();

            let res = self
                .storage
                .get_scripts(
                    vec![lock_hash.clone()],
                    vec![],
                    None,
                    vec![],
                    PaginationRequest::default().set_limit(Some(1)),
                )
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if res.response.is_empty() {
                return Ok(AddressOrLockHash::LockHash(lock_hash.to_string()));
            } else {
                return Ok(AddressOrLockHash::Address(
                    self.script_to_address(res.response.get(0).unwrap())
                        .to_string(),
                ));
            }
        }

        Ok(AddressOrLockHash::Address(
            self.script_to_address(&cell.cell_output.lock()).to_string(),
        ))
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

    async fn generate_udt_address_or_lock_hash(
        &self,
        cell: &DetailedCell,
        io_type: &IOType,
        tip_epoch_number: &RationalU256,
    ) -> InnerResult<AddressOrLockHash> {
        let lock_code_hash: H256 = cell.cell_output.lock().code_hash().unpack();

        if lock_code_hash == **SECP256K1_CODE_HASH.load()
            || lock_code_hash == **ACP_CODE_HASH.load()
        {
            return Ok(AddressOrLockHash::Address(
                self.script_to_address(&cell.cell_output.lock()).to_string(),
            ));
        }

        if lock_code_hash == **CHEQUE_CODE_HASH.load() {
            let generate_epoch_num;
            let judge_epoch_num;

            if io_type == &IOType::Input {
                generate_epoch_num = self
                    .storage
                    .get_transaction_info_by_hash(cell.out_point.tx_hash().unpack())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .epoch_number;
                judge_epoch_num = RationalU256::from_u256(cell.epoch_number.clone());
            } else {
                let res = self
                    .storage
                    .get_spent_transaction_hash(cell.out_point.clone())
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                generate_epoch_num = RationalU256::from_u256(cell.epoch_number.clone());

                judge_epoch_num = if let Some(hash) = res {
                    let tx_info = self
                        .storage
                        .get_transaction_info_by_hash(hash)
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                    tx_info.epoch_number
                } else {
                    tip_epoch_number.clone()
                };
            }

            let lock_hash_160 = if self.is_unlock(
                generate_epoch_num,
                Some(judge_epoch_num),
                self.cheque_timeout.clone(),
            ) {
                cell.cell_output.lock().args().raw_data()[20..40].to_vec()
            } else {
                cell.cell_output.lock().args().raw_data()[0..20].to_vec()
            };
            let lock_hash = H160::from_slice(&lock_hash_160).unwrap();

            let res = self
                .storage
                .get_scripts(
                    vec![lock_hash.clone()],
                    vec![],
                    None,
                    vec![],
                    PaginationRequest::default().set_limit(Some(1)),
                )
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if res.response.is_empty() {
                return Ok(AddressOrLockHash::LockHash(lock_hash.to_string()));
            } else {
                return Ok(AddressOrLockHash::Address(
                    self.script_to_address(res.response.get(0).unwrap())
                        .to_string(),
                ));
            }
        }

        Ok(AddressOrLockHash::Address(
            self.script_to_address(&cell.cell_output.lock()).to_string(),
        ))
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
        tip_epoch_number: &RationalU256,
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
            let res = self
                .storage
                .get_spent_transaction_hash(cell.out_point.clone())
                .await
                .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
            if let Some(hash) = res {
                let tx_info = self
                    .storage
                    .get_transaction_info_by_hash(hash)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                Ok(Status::Fixed(tx_info.block_number))
            } else if self.is_unlock(
                RationalU256::from_u256(cell.epoch_number.clone()),
                Some(tip_epoch_number.clone()),
                self.cheque_timeout.clone(),
            ) {
                let mut timeout_block_num = cell.block_number;
                timeout_block_num += 180 * 6;

                Ok(Status::Fixed(timeout_block_num))
            } else {
                Ok(Status::Claimable(cell.block_number))
            }
        } else {
            Err(RpcErrorMessage::UnsupportUDTLockScript)
        }
    }

    async fn generate_extra(
        &self,
        cell: &DetailedCell,
        io_type: IOType,
        tip_block_number: BlockNumber,
    ) -> InnerResult<Option<ExtraFilter>> {
        if cell.tx_index == 0 && io_type == IOType::Output {
            return Ok(Some(ExtraFilter::CellBase));
        }

        if let Some(type_script) = cell.cell_output.type_().to_opt() {
            let type_code_hash: H256 = type_script.code_hash().unpack();

            if type_code_hash == **DAO_CODE_HASH.load() {
                let block_num = if io_type == IOType::Input {
                    self.storage
                        .get_transaction_info_by_hash(cell.out_point.tx_hash().unpack())
                        .await
                        .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                        .block_number
                } else {
                    cell.block_number
                };

                let (state, start_hash, end_hash) =
                    if cell.cell_data == Bytes::from(vec![0, 0, 0, 0]) {
                        let tip_hash = self
                            .storage
                            .get_canonical_block_hash(tip_block_number)
                            .await
                            .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                        (
                            DaoState::Deposit(block_num),
                            cell.block_hash.clone(),
                            tip_hash,
                        )
                    } else {
                        let deposit_block_num = decode_dao_block_number(&cell.cell_data);
                        let tmp_hash = self
                            .storage
                            .get_canonical_block_hash(deposit_block_num)
                            .await
                            .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?;
                        (
                            DaoState::Withdraw(deposit_block_num, block_num),
                            tmp_hash,
                            cell.block_hash.clone(),
                        )
                    };

                let start_ar = self
                    .storage
                    .get_block_header(Some(start_hash), None)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .dao()
                    .raw_data()[8..15]
                    .to_vec();
                let start_ar = decode_dao_block_number(&start_ar);
                let end_ar = self
                    .storage
                    .get_block_header(Some(end_hash), None)
                    .await
                    .map_err(|e| RpcErrorMessage::DBError(e.to_string()))?
                    .dao()
                    .raw_data()[8..15]
                    .to_vec();
                let end_ar = decode_dao_block_number(&end_ar);

                let capacity: u64 = cell.cell_output.capacity().unpack();
                let reward = capacity * end_ar / start_ar - capacity;

                return Ok(Some(ExtraFilter::Dao(DaoInfo { state, reward })));
            }
        }
        Ok(None)
    }

    pub(crate) async fn get_pool_live_cells_by_item(
        &self,
        item: Item,
        required_ckb: i64,
        required_udts: Vec<RequiredUDT>,
        source: Option<Source>,
        script_set: &mut HashSet<packed::Script>,
        sig_entries: &mut HashMap<String, SignatureEntry>,
    ) -> InnerResult<Vec<DetailedCell>> {
        let mut pool_cells = Vec::new();
        let zero = BigInt::from(0);
        let item_lock_hash = self.get_secp_lock_hash_by_item(item.clone())?;

        self.pool_udt(
            required_udts,
            &item,
            source,
            &mut pool_cells,
            item_lock_hash,
            &zero,
        )
        .await?;

        let ckb_collect_already = pool_cells
            .iter()
            .map::<u64, _>(|cell| cell.cell_output.capacity().unpack())
            .sum::<u64>();
        let mut required_ckb = BigInt::from(required_ckb) - ckb_collect_already;

        if required_ckb <= zero {
            return Ok(pool_cells);
        }

        let asset_ckb = AssetInfo::new_ckb();

        // TODO
        let mut asset_ckb_set = HashSet::new();
        asset_ckb_set.insert(asset_ckb.clone());
        let dao_cells = self
            .get_live_cells_by_item(
                item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                Some((**SECP256K1_CODE_HASH.load()).clone()),
                Some(ExtraFilter::Dao(DaoInfo::new_deposit(0, 0))),
            )
            .await?;

        let dao_cells = dao_cells
            .into_iter()
            .filter(|cell| is_dao_unlock(&cell))
            .collect::<Vec<_>>();

        if self.pool_asset(&mut pool_cells, &mut required_ckb, dao_cells, true) {
            return Ok(pool_cells);
        }

        let cell_base_cells = self
            .get_live_cells_by_item(
                item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                Some((**SECP256K1_CODE_HASH.load()).clone()),
                Some(ExtraFilter::CellBase),
            )
            .await?;
        let cell_base_cells = cell_base_cells
            .into_iter()
            .filter(|cell| self.is_cellbase_mature(&cell))
            .collect::<Vec<_>>();

        if self.pool_asset(&mut pool_cells, &mut required_ckb, cell_base_cells, true) {
            return Ok(pool_cells);
        }

        let normal_ckb_cells = self
            .get_live_cells_by_item(
                item.clone(),
                asset_ckb_set.clone(),
                None,
                None,
                Some((**SECP256K1_CODE_HASH.load()).clone()),
                None,
            )
            .await?;
        let normal_ckb_cells = normal_ckb_cells
            .into_iter()
            .filter(|cell| cell.cell_data.is_empty())
            .collect::<Vec<_>>();

        if self.pool_asset(&mut pool_cells, &mut required_ckb, normal_ckb_cells, true) {
            return Ok(pool_cells);
        }

        if required_ckb > zero {
            return Err(RpcErrorMessage::TokenIsNotEnough(asset_ckb.to_string()));
        }

        Ok(pool_cells)
    }

    pub(crate) async fn try_pool_ckb_by_items(
        &self,
        item_set: Vec<Item>,
        required_ckb: i64,
    ) -> InnerResult<Vec<DetailedCell>> {
        let zero = BigInt::from(0);
        let asset_ckb = AssetInfo::new_ckb();
        let mut required_ckb = BigInt::from(required_ckb);
        let mut pool_cells = Vec::new();

        for item in item_set.iter() {
            // TODO
            let dao_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_ckb.clone(),
                    Some((**SECP256K1_CODE_HASH.load()).clone()),
                    Some(ExtraFilter::Dao(DaoInfo::new_deposit(0, 0))),
                )
                .await?;

            let dao_cells = dao_cells
                .into_iter()
                .filter(|cell| is_dao_unlock(&cell))
                .collect::<Vec<_>>();

            if self.pool_asset(&mut pool_cells, &mut required_ckb, dao_cells, true) {
                return Ok(pool_cells);
            }
        }

        for item in item_set.iter() {
            let cell_base_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_ckb.clone(),
                    Some((**SECP256K1_CODE_HASH.load()).clone()),
                    Some(ExtraFilter::CellBase),
                )
                .await?;
            let cell_base_cells = cell_base_cells
                .into_iter()
                .filter(|cell| self.is_cellbase_mature(&cell))
                .collect::<Vec<_>>();

            if self.pool_asset(&mut pool_cells, &mut required_ckb, cell_base_cells, true) {
                return Ok(pool_cells);
            }
        }

        for item in item_set.iter() {
            let normal_ckb_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_ckb.clone(),
                    Some((**SECP256K1_CODE_HASH.load()).clone()),
                    None,
                )
                .await?;
            let normal_ckb_cells = normal_ckb_cells
                .into_iter()
                .filter(|cell| cell.cell_data.is_empty())
                .collect::<Vec<_>>();

            if self.pool_asset(&mut pool_cells, &mut required_ckb, normal_ckb_cells, true) {
                return Ok(pool_cells);
            }
        }

        if required_ckb > zero {
            return Err(RpcErrorMessage::TokenIsNotEnough(
                AssetInfo::new_ckb().to_string(),
            ));
        }

        Ok(pool_cells)

    pub(crate) async fn accumulate_balance_from_records(
        &self,
        balances_map: &mut HashMap<(AddressOrLockHash, AssetInfo), Balance>,
        records: &[Record],
        tip_epoch_number: &RationalU256,
    ) -> InnerResult<()> {
        for record in records {
            let key = (
                record.address_or_lock_hash.clone(),
                record.asset_info.clone(),
            );

            let mut balance = match balances_map.get(&key) {
                Some(balance) => balance.clone(),
                None => Balance::new(
                    record.address_or_lock_hash.clone(),
                    record.asset_info.clone(),
                ),
            };

            let amount = u128::from_str(&record.amount).unwrap();
            let occupied = record.occupied as u128 + u128::from_str(&balance.occupied).unwrap();
            let freezed = match &record.extra {
                Some(ExtraFilter::Dao(dao_info)) => match dao_info.state {
                    DaoState::Deposit(_) => amount - occupied,
                    DaoState::Withdraw(deposit_block_number, withdraw_block_number) => {
                        let deposit_epoch = self.get_epoch_by_number(deposit_block_number).await?;
                        let withdraw_epoch =
                            self.get_epoch_by_number(withdraw_block_number).await?;
                        if self.is_dao_withdraw_unlock(
                            deposit_epoch,
                            withdraw_epoch,
                            Some(tip_epoch_number.clone()),
                        ) {
                            0u128
                        } else {
                            amount - occupied
                        }
                    }
                },
                Some(ExtraFilter::CellBase) => {
                    let block_number = match &record.status {
                        Status::Claimable(_) => unreachable!(),
                        Status::Fixed(block_number) => block_number,
                    };
                    let epoch_number = self.get_epoch_by_number(*block_number).await?;
                    if self.is_unlock(
                        epoch_number,
                        Some(tip_epoch_number.clone()),
                        self.cellbase_maturity.clone(),
                    ) {
                        0u128
                    } else {
                        amount - occupied
                    }
                }
                None => 0u128,
            } + u128::from_str(&balance.freezed).unwrap();
            let claimable = match &record.status {
                Status::Claimable(_) => amount,
                _ => 0u128,
            } + u128::from_str(&balance.claimable).unwrap();
            let free =
                amount - occupied - freezed - claimable + u128::from_str(&balance.free).unwrap();

            balance.free = free.to_string();
            balance.occupied = occupied.to_string();
            balance.freezed = freezed.to_string();
            balance.claimable = claimable.to_string();

            balances_map.insert(key, balance.clone());
        }

        Ok(())
    }

    pub(crate) async fn get_epoch_by_number(
        &self,
        block_number: BlockNumber,
    ) -> InnerResult<RationalU256> {
        let header = self
            .storage
            .get_block_header(None, Some(block_number))
            .await
            .map_err(|_| RpcErrorMessage::GetEpochFromNumberError(block_number))?;
        Ok(header.epoch().to_rational())
    }

    async fn pool_udt(
        &self,
        required_udts: Vec<RequiredUDT>,
        item: &Item,
        source: Option<Source>,
        pool_cells: &mut Vec<DetailedCell>,
        item_lock_hash: H160,
        zero: &BigInt,
    ) -> InnerResult<()> {
        for required_udt in required_udts.iter() {
            let asset_info = AssetInfo::new_udt(required_udt.udt_hash.clone());
            let mut asset_udt_set = HashSet::new();
            asset_udt_set.insert(asset_info.clone());
            let mut udt_required = BigInt::from(required_udt.amount_required);
            let cheque_cells = self
                .get_live_cells_by_item(
                    item.clone(),
                    asset_udt_set.clone(),
                    None,
                    None,
                    Some((**CHEQUE_CODE_HASH.load()).clone()),
                    None,
                )
                .await?;

            if source.is_none() || source == Some(Source::Claimable) {
                let cheque_cells_in_time = cheque_cells
                    .clone()
                    .into_iter()
                    .filter(|cell| {
                        let receiver_lock_hash =
                            H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                                .unwrap();

                        receiver_lock_hash == item_lock_hash
                    })
                    .collect::<Vec<_>>();

                if self.pool_asset(pool_cells, &mut udt_required, cheque_cells_in_time, false) {
                    break;
                }
            }

            if source.is_none() || source == Some(Source::Free) {
                let cheque_cells_time_out = cheque_cells
                    .into_iter()
                    .filter(|cell| {
                        let sender_lock_hash =
                            H160::from_slice(&cell.cell_output.lock().args().raw_data()[0..20])
                                .unwrap();
                        sender_lock_hash == item_lock_hash
                    })
                    .collect();

                if self.pool_asset(pool_cells, &mut udt_required, cheque_cells_time_out, false) {
                    break;
                }

                let secp_cells = self
                    .get_live_cells_by_item(
                        item.clone(),
                        asset_udt_set.clone(),
                        None,
                        None,
                        Some((**SECP256K1_CODE_HASH.load()).clone()),
                        None,
                    )
                    .await?;

                if self.pool_asset(pool_cells, &mut udt_required, secp_cells, false) {
                    break;
                }

                let acp_cells = self
                    .get_live_cells_by_item(
                        item.clone(),
                        asset_udt_set.clone(),
                        None,
                        None,
                        Some((**ACP_CODE_HASH.load()).clone()),
                        None,
                    )
                    .await?;

                if self.pool_asset(pool_cells, &mut udt_required, acp_cells, false) {
                    break;
                }
            }

            if udt_required > *zero {
                return Err(RpcErrorMessage::TokenIsNotEnough(asset_info.to_string()));
            }
        }

        Ok(())
    }

    fn filter_useless_cheque(
        &self,
        cell: &DetailedCell,
        secp_lock_hash: &H256,
        tip_epoch_number: Option<RationalU256>,
    ) -> bool {
        let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if code_hash == **CHEQUE_CODE_HASH.load() {
            let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();

            if self.is_unlock(
                RationalU256::from_u256(cell.epoch_number.clone()),
                tip_epoch_number,
                self.cheque_timeout.clone(),
            ) {
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

    pub(crate) fn is_unlock(
        &self,
        from: RationalU256,
        end: Option<RationalU256>,
        unlock_gap: RationalU256,
    ) -> bool {
        if let Some(end) = end {
            end.saturating_sub(from) > unlock_gap
        } else {
            (**CURRENT_EPOCH_NUMBER.load()).clone().saturating_sub(from) > unlock_gap
        }
    }

    pub(crate) fn is_dao_withdraw_unlock(
        &self,
        deposit_epoch: RationalU256,
        withdraw_epoch: RationalU256,
        tip_epoch: Option<RationalU256>,
    ) -> bool {
        let deposit_duration = withdraw_epoch - deposit_epoch;
        let dao_cycle = RationalU256::from_u256(180u64.into());
        let cycle_count = deposit_duration / dao_cycle.clone();
        let unlock_epoch = dao_cycle * (cycle_count + RationalU256::one());

        if let Some(tip_epoch) = tip_epoch {
            tip_epoch > unlock_epoch
        } else {
            *CURRENT_EPOCH_NUMBER.load().clone() > unlock_epoch
        }
    }

    pub(crate) fn script_to_address(&self, script: &packed::Script) -> Address {
        let payload = AddressPayload::from_script(script, self.network_type);
        Address::new(self.network_type, payload)
    }
    fn is_cellbase_mature(&self, cell: &DetailedCell) -> bool {
        (**CURRENT_EPOCH_NUMBER.load())
            .clone()
            .saturating_sub_u256(cell.epoch_number.clone())
            > self.cellbase_maturity
    }
}

fn is_dao_unlock(_cell: &DetailedCell) -> bool {
    true
}
