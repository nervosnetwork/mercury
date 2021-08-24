use crate::rpc_impl::{
    address_to_script, ACP_CODE_HASH, CHEQUE_CODE_HASH, CURRENT_BLOCK_NUMBER, CURRENT_EPOCH_NUMBER,
    DAO_CODE_HASH, SECP256K1_CODE_HASH,
};
use crate::types::{
    decode_record_id, encode_record_id, AssetType, ExtraFilter, Identity, IdentityFlag, Item,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::utils::parse_address;
use common::{
    anyhow::Result, Address, DetailedCell, Order, PaginationRequest, PaginationResponse, Range,
    ACP, CHEQUE, DAO, SECP256K1,
};
use core_storage::DBAdapter;

use ckb_types::core::{BlockNumber, RationalU256};
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};

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
    ) -> Result<Vec<packed::Script>> {
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
    ) -> Result<Vec<packed::Script>> {
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
    ) -> Result<Vec<DetailedCell>> {
        let type_hashes = match asset_type {
            AssetType::Ckb => match extra {
                Some(ExtraFilter::Dao) => vec![self
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
                    .get_live_cells(lock_hashes, type_hashes, None, range, pagination)
                    .await?;
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
                    .get_live_cells(lock_hashes, type_hashes, None, range, pagination)
                    .await?;

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
                let (_outpoint, address) = decode_record_id(id);
                let mut lock_hashes = vec![];
                if lock_filter.is_some() {
                    lock_hashes.push(lock_filter.unwrap());
                }
                // get_live_cells 需要增加对 outpoint 查询的支持
                let cell = self
                    .storage
                    .get_live_cells(lock_hashes, type_hashes, None, range, pagination)
                    .await?;

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
                        .is_cheque_timeout(RationalU256::from_u256(cell.epoch_number.clone()))
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

    fn filter_useless_cheque(&self, cell: &DetailedCell, secp_lock_hash: &H256) -> bool {
        let code_hash: H256 = cell.cell_output.lock().code_hash().unpack();
        if code_hash == **CHEQUE_CODE_HASH.load() {
            let cell_args: Vec<u8> = cell.cell_output.lock().args().unpack();

            if self.is_cheque_timeout(RationalU256::from_u256(cell.epoch_number.clone())) {
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

    pub(crate) fn is_cheque_timeout(&self, epoch_num: RationalU256) -> bool {
        &*CURRENT_EPOCH_NUMBER.load().clone() - epoch_num > self.cheque_since
    }
}
