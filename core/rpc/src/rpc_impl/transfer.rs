use crate::rpc_impl::{
    address_to_script, ckb_iter, parse_key_address, parse_normal_address, udt_iter, MercuryRpcImpl,
    ACP_USED_CACHE, BYTE_SHANNONS, CHEQUE_CELL_CAPACITY, INIT_ESTIMATE_FEE, MIN_CKB_CAPACITY,
    STANDARD_SUDT_CAPACITY, TX_POOL_CACHE,
};
use crate::types::{
    details_split_off, Action, CellWithData, DetailedAmount, FromAddresses, FromKeyAddresses,
    InnerAccount, InnerTransferItem, InputConsume, ScriptType, SignatureEntry, SignatureType,
    Source, ToAddress, ToKeyAddress, TransactionCompletionResponse, WalletInfo, WitnessType,
    CHEQUE, SECP256K1,
};
use crate::{error::RpcError, CkbRpc};

use common::utils::{
    decode_udt_amount, encode_udt_amount, parse_address, to_fixed_array, u128_sub, unwrap_only_one,
};
use common::{anyhow::Result, hash::blake2b_160, Address, AddressPayload, MercuryError};
use core_extensions::{special_cells, udt_balance, DetailedCell, CURRENT_EPOCH, UDT_EXT_PREFIX};

use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_jsonrpc_types::TransactionView as JsonTransactionView;
use ckb_types::core::{RationalU256, ScriptHashType, TransactionBuilder, TransactionView};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256};
use num_bigint::BigUint;
use num_traits::identities::Zero;

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, ops::Sub, thread};

impl<S, C> MercuryRpcImpl<S, C>
where
    S: Store,
    C: CkbRpc + Clone + Send + Sync + 'static,
{
    pub(crate) fn inner_transfer_complete(
        &self,
        udt_hash: Option<H256>,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee_rate: u64,
    ) -> Result<TransactionCompletionResponse> {
        let mut estimate_fee = INIT_ESTIMATE_FEE;
        loop {
            let response = self.inner_transfer_complete_with_fixed_fee(
                udt_hash.clone(),
                from.clone(),
                items.clone(),
                change.clone(),
                estimate_fee,
            )?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.sigs_entry.clone(),
            );
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let change = change.unwrap_or_else(|| from.idents[0].clone());
                let change_address = parse_address(&change).unwrap();
                let tx_view = self.update_tx_view_change_cell(
                    response.tx_view,
                    change_address,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.sigs_entry);
                return Ok(adjust_response);
            }
        }
    }

    pub(crate) fn inner_transfer_complete_with_fixed_fee(
        &self,
        udt_hash: Option<H256>,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee: u64,
    ) -> Result<TransactionCompletionResponse> {
        let mut amounts = DetailedAmount::new();
        let mut scripts_set = from
            .scripts
            .iter()
            .map(|s| s.as_str().to_string())
            .collect::<HashSet<_>>();
        let (mut inputs, mut outputs, mut cell_data) = (vec![], vec![], vec![]);
        let change = change.unwrap_or_else(|| from.idents[0].clone());

        if udt_hash.is_some() {
            scripts_set.insert(ScriptType::SUDT.as_str().to_string());
        }

        for item in items.iter() {
            let addr = unwrap_only_one(&item.to.idents);
            let script = unwrap_only_one(&item.to.scripts);
            scripts_set.insert(script.as_str().to_string());
            let (amount_ckb, amount_udt) = if udt_hash.is_none() {
                (item.amount as u64, 0u128)
            } else {
                (0u64, item.amount)
            };

            let output_cells = self.build_outputs(
                &udt_hash,
                &parse_address(&addr)?,
                amount_ckb,
                amount_udt,
                &script,
                &mut amounts,
                from.idents[0].clone(),
            )?;

            outputs.push(output_cells.cell);
            cell_data.push(output_cells.data);
        }

        let (consume, sigs_entry) = self.build_inputs(
            &udt_hash,
            from,
            &amounts,
            fee,
            &mut inputs,
            &mut outputs,
            &mut cell_data,
            &mut scripts_set,
        )?;

        // The ckb and udt needed must be zero here. If the consumed udt is
        // smaller than the udt amount in tx output, it will use acp cell to
        // pay for udt.
        let (mut change_cell, mut change_data) = self.build_change_cell(
            change,
            udt_hash,
            consume.ckb - amounts.ckb_all - fee,
            u128_sub(consume.udt, amounts.udt_amount.into()),
        )?;
        let cell_deps = self.build_cell_deps(scripts_set);

        outputs.append(&mut change_cell);
        cell_data.append(&mut change_data);

        let view = self.build_tx_view(cell_deps, inputs, outputs, cell_data);

        Ok(TransactionCompletionResponse::new(view.into(), sigs_entry))
    }

    pub(crate) fn inner_create_wallet(
        &self,
        address: Address,
        udt_info: Vec<WalletInfo>,
        fee_rate: u64,
    ) -> Result<TransactionCompletionResponse> {
        let mut estimate_fee = INIT_ESTIMATE_FEE;
        loop {
            let response = self.inner_create_wallet_with_fixed_fee(
                address.clone(),
                udt_info.clone(),
                estimate_fee,
            )?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.sigs_entry.clone(),
            );
            let actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let change_address = address;
                let tx_view = self.update_tx_view_change_cell(
                    response.tx_view,
                    change_address,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.sigs_entry);
                return Ok(adjust_response);
            }
        }
    }

    pub(crate) fn inner_create_wallet_with_fixed_fee(
        &self,
        address: Address,
        udt_info: Vec<WalletInfo>,
        fee: u64,
    ) -> Result<TransactionCompletionResponse> {
        let mut capacity_needed = fee + MIN_CKB_CAPACITY;
        let (mut inputs, mut outputs, mut sigs_entry) = (vec![], vec![], HashMap::new());
        let addr_payload = address.payload().to_owned();
        let pubkey_hash = addr_payload.args();
        let lock_script = address_to_script(&addr_payload);
        let acp_lock = self
            .config
            .get(special_cells::ACP)
            .ok_or_else(|| {
                MercuryError::rpc(RpcError::MissingConfig(special_cells::ACP.to_string()))
            })?
            .clone()
            .script;

        for info in udt_info.iter() {
            info.check()?;
            let (udt_script, data) = self.build_type_script(Some(info.udt_hash.clone()), 0)?;
            let capacity = info.expected_capacity();
            let lock_args =
                self.build_acp_lock_args(pubkey_hash.clone(), info.min_ckb, info.min_udt)?;
            let cell = packed::CellOutputBuilder::default()
                .type_(udt_script.pack())
                .lock(acp_lock.clone().as_builder().args(lock_args.pack()).build())
                .capacity(capacity.pack())
                .build();

            capacity_needed += capacity;
            outputs.push(CellWithData::new(cell, data));
        }

        let cells = self.get_cells_by_lock_script(&lock_script)?;
        let mut ckb_needed = BigUint::from(capacity_needed);
        let mut capacity_sum = 0u64;
        self.pool_ckb(
            ckb_iter(&cells),
            &mut ckb_needed,
            &mut inputs,
            &mut sigs_entry,
            &mut capacity_sum,
        );

        if ckb_needed > Zero::zero() {
            return Err(MercuryError::rpc(RpcError::CkbIsNotEnough(address.to_string())).into());
        }

        outputs.push(CellWithData::new(
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .capacity((capacity_sum - capacity_needed + MIN_CKB_CAPACITY).pack())
                .build(),
            Default::default(),
        ));

        let scripts = vec![
            SECP256K1.to_string(),
            special_cells::ACP.to_string(),
            udt_balance::SUDT.to_string(),
        ]
        .into_iter()
        .collect::<HashSet<_>>();

        let cell_deps = self.build_cell_deps(scripts);
        let (mut outputs_cell, mut outputs_data) = (vec![], vec![]);
        details_split_off(outputs, &mut outputs_cell, &mut outputs_data);

        let tx_view = self.build_tx_view(cell_deps, inputs, outputs_cell, outputs_data);
        let mut sigs_entry = sigs_entry.into_iter().map(|(_k, v)| v).collect::<Vec<_>>();
        sigs_entry.sort();

        Ok(TransactionCompletionResponse::new(
            tx_view.into(),
            sigs_entry,
        ))
    }

    pub(crate) fn inner_collect_asset(
        &self,
        from: FromAddresses,
        to: ToAddress,
        udt_hash: Option<H256>,
        fee_paid_by: String,
        fee_rate: u64,
    ) -> Result<TransactionCompletionResponse> {
        let mut estimate_fee = INIT_ESTIMATE_FEE;
        loop {
            let response = self.inner_collect_asset_with_fixed_fee(
                from.clone(),
                udt_hash.clone(),
                to.clone(),
                fee_paid_by.clone(),
                estimate_fee,
            )?;
            let tx_size = calculate_tx_size_with_witness_placeholder(
                response.tx_view.clone(),
                response.sigs_entry.clone(),
            );
            let mut actual_fee = fee_rate.saturating_mul(tx_size as u64) / 1000;
            if actual_fee * 1000 < fee_rate.saturating_mul(tx_size as u64) {
                actual_fee += 1;
            }
            if estimate_fee < actual_fee {
                // increase estimate fee by 1 CKB
                estimate_fee += BYTE_SHANNONS;
                continue;
            } else {
                let fee_address = parse_address(&fee_paid_by)?;
                let tx_view = self.update_tx_view_change_cell(
                    response.tx_view,
                    fee_address,
                    estimate_fee,
                    actual_fee,
                )?;
                let adjust_response =
                    TransactionCompletionResponse::new(tx_view, response.sigs_entry);
                return Ok(adjust_response);
            }
        }
    }

    pub(crate) fn inner_collect_asset_with_fixed_fee(
        &self,
        from: FromAddresses,
        udt_hash: Option<H256>,
        to: ToAddress,
        fee_paid_by: String,
        fee: u64,
    ) -> Result<TransactionCompletionResponse> {
        let from_addresses = self.parse_from_addresses(from, udt_hash.is_some())?;
        let to_address = self.parse_to_address(to, udt_hash.is_some())?;
        let fee_address = parse_address(&fee_paid_by)?;

        let (mut inputs, mut sigs_entry, mut script_type_set, mut outputs, mut cell_data) =
            match udt_hash {
                Some(udt_hash) => {
                    let (inputs, sigs_entry, outputs, cell_data) = self
                        .build_inputs_outputs_for_asset_collection_udt(
                            from_addresses,
                            to_address,
                            udt_hash,
                        )?;
                    let mut script_type_set = HashSet::new();
                    script_type_set.insert(ScriptType::Secp256k1.as_str().to_string());
                    script_type_set.insert(ScriptType::Cheque.as_str().to_string());
                    script_type_set.insert(ScriptType::AnyoneCanPay.as_str().to_string());
                    (inputs, sigs_entry, script_type_set, outputs, cell_data)
                }
                None => {
                    let (ckb_consumed, mut inputs, mut sigs_entry) =
                        self.build_inputs_for_asset_collection_ckb(from_addresses)?;
                    let output =
                        self.build_output_for_asset_collection(to_address, ckb_consumed, udt_hash);
                    let outputs = vec![output];
                    let mut script_type_set = HashSet::new();
                    script_type_set.insert(ScriptType::Secp256k1.as_str().to_string());
                    let cell_data = vec![Default::default()];
                    (inputs, sigs_entry, script_type_set, outputs, cell_data)
                }
            };
        // handle fee payment
        let (fee_input, fee_output) = self.pay_fee(fee_address.clone(), fee)?;
        inputs.push(fee_input);
        sigs_entry.push(SignatureEntry::new(
            inputs.len(),
            fee_address.to_string(),
            SignatureType::Secp256k1,
        ));
        outputs.push(fee_output);
        let cell_deps = self.build_cell_deps(script_type_set);
        cell_data.push(Default::default());
        let view = self.build_tx_view(cell_deps, inputs, outputs, cell_data);
        Ok(TransactionCompletionResponse::new(view.into(), sigs_entry))
    }

    fn parse_from_addresses(&self, from: FromAddresses, is_udt: bool) -> Result<Vec<Address>> {
        match from {
            FromAddresses::KeyAddresses(FromKeyAddresses {
                key_addresses,
                source,
            }) => {
                // when collect CKB, the source must be Unconstrained
                // when collect UDT, the source must be Fleeting
                if is_udt && source != Source::Fleeting
                    || !is_udt && source != Source::Unconstrained
                {
                    return Err(MercuryError::rpc(RpcError::UnsupportedSource).into());
                }
                key_addresses
                    .iter()
                    .map(|addr| parse_key_address(addr))
                    .collect::<Result<Vec<_>, _>>()
            }
            FromAddresses::NormalAddresses(normal_addresses) => {
                if is_udt {
                    let mut addresses: Vec<Address> = vec![];
                    for addr in normal_addresses {
                        let normal_addr = parse_normal_address(&addr)?;
                        let script = address_to_script(normal_addr.payload());
                        if self.is_cheque(&script) {
                            let key_addr = Address::new(
                                self.net_ty,
                                self.get_script_by_hash(to_fixed_array(
                                    &script.args().raw_data()[20..40],
                                ))?
                                .into(),
                            );
                            addresses.push(key_addr);
                        } else {
                            return Err(
                                MercuryError::rpc(RpcError::InvalidNormalAddress(addr)).into()
                            );
                        }
                    }
                    Ok(addresses)
                } else {
                    // when collect CKB, from_addresses must be secp256k1 lock
                    normal_addresses
                        .iter()
                        .map(|addr| parse_key_address(addr))
                        .collect::<Result<Vec<_>, _>>()
                }
            }
        }
    }

    fn parse_to_address(&self, to: ToAddress, is_udt: bool) -> Result<Address> {
        match to {
            ToAddress::KeyAddress(ToKeyAddress {
                key_address,
                action,
            }) => {
                // when collect CKB, to_address must be secp lock script with action PayByFrom
                // when collect UDT, to_address must be acp lock script with action PayByTo
                if is_udt && action != Action::PayByTo || !is_udt && action != Action::PayByFrom {
                    return Err(MercuryError::rpc(RpcError::UnsupportedAction).into());
                }
                parse_key_address(&key_address)
            }
            ToAddress::NormalAddress(normal_address) => {
                if is_udt {
                    let addr = parse_normal_address(&normal_address)?;
                    let script = address_to_script(&addr.payload());
                    if self.is_acp(&script) {
                        let key_addr = self.pubkey_to_key_address(H160::from_slice(
                            &script.args().raw_data()[0..20],
                        )?);
                        Ok(key_addr)
                    } else {
                        Err(
                            MercuryError::rpc(RpcError::InvalidNormalAddress(addr.to_string()))
                                .into(),
                        )
                    }
                } else {
                    parse_key_address(&normal_address)
                }
            }
        }
    }

    fn build_inputs_outputs_for_asset_collection_udt(
        &self,
        from_addresses: Vec<Address>,
        to_address: Address,
        udt_hash: H256,
    ) -> Result<(
        Vec<packed::OutPoint>,
        Vec<SignatureEntry>,
        Vec<packed::CellOutput>,
        Vec<packed::Bytes>,
    )> {
        // inputs: (n*m)cheque cell + 1 acp cell
        // outputs: 1 acp cell(receiver udt) + n secp cell(sender ckb)
        // sigs_entry: n secp sig(by receiver) grouped
        let mut all_out_points = vec![];
        let mut all_cheque_cells = vec![];
        let mut sigs_entry = vec![];
        let mut cell_outputs = vec![];
        let mut cell_data = vec![];
        for address in from_addresses {
            let script = address_to_script(&address.payload());
            let mut cells = self.collect_claimable_cells_for_asset_collection_udt(
                address.clone(),
                udt_hash.clone(),
            )?;
            let mut out_points = cells
                .iter()
                .map(|cell| cell.out_point.to_owned())
                .collect::<Vec<_>>();
            sigs_entry.push(SignatureEntry {
                type_: WitnessType::WitnessArgsLock,
                index: all_out_points.len(),
                group_len: out_points.len(),
                pub_key: address.to_string(),
                sig_type: SignatureType::Secp256k1,
            });
            all_out_points.append(&mut out_points);
            all_cheque_cells.append(&mut cells);
            for cell in cells {
                let lock_args = cell.cell_output.lock().args().raw_data();
                assert_eq!(lock_args.len(), 40);
                let sender_script_hash = lock_args[20..40].try_into()?;
                let script = self.get_script_by_hash(sender_script_hash)?;
                let cell_output = packed::CellOutputBuilder::default()
                    .lock(script)
                    .capacity(cell.cell_output.capacity())
                    .build();
                cell_outputs.push(cell_output);
                cell_data.push(Default::default());
            }
        }
        let udt_consumed = all_cheque_cells
            .iter()
            .map(|cell| u128::from_le_bytes(to_fixed_array(&cell.cell_data.raw_data()[0..16])))
            .sum::<u128>();
        // consume an acp cell, generate a new acp cell plus udt_consumed
        let config = self.get_config(special_cells::ACP)?;
        let acp_lock_args = self.build_acp_lock_args(to_address.payload().args(), None, None)?;
        let acp_script = packed::ScriptBuilder::default()
            .code_hash(config.code_hash())
            .hash_type(ScriptHashType::Type.into())
            .args(acp_lock_args.pack())
            .build();
        let input_acp_cell = self.find_live_acp_cell(&to_address, &udt_hash)?;
        let output_acp_cell = input_acp_cell.cell_output;
        let new_acp_udt_amount =
            decode_udt_amount(&input_acp_cell.cell_data.raw_data()) + udt_consumed;
        let new_acp_cell_data = u128::to_le_bytes(new_acp_udt_amount).to_vec().pack();
        all_out_points.push(input_acp_cell.out_point);
        cell_outputs.push(output_acp_cell);
        Ok((all_out_points, sigs_entry, cell_outputs, cell_data))
    }

    fn find_live_acp_cell(&self, address: &Address, udt_hash: &H256) -> Result<DetailedCell> {
        let sp_cells = self.get_sp_cells_by_addr(&address)?.inner();
        let acp_cells = self.take_sp_cells(&sp_cells, special_cells::ACP)?;
        acp_cells
            .iter()
            .find(|cell| {
                cell.cell_output.type_().is_some()
                    && cell
                        .cell_output
                        .type_()
                        .to_opt()
                        .unwrap()
                        .calc_script_hash()
                        == udt_hash.pack()
            })
            .cloned()
            .ok_or_else(|| {
                MercuryError::rpc(RpcError::MissingACPCell(
                    address.to_string(),
                    hex::encode(udt_hash.as_ref()),
                ))
                .into()
            })
    }

    fn build_inputs_for_asset_collection_ckb(
        &self,
        from_addresses: Vec<Address>,
    ) -> Result<(u64, Vec<packed::OutPoint>, Vec<SignatureEntry>)> {
        let mut all_ckb_cells = vec![];
        let mut all_out_points = vec![];
        let mut sigs_entry = vec![];
        for address in from_addresses {
            let script = address_to_script(address.payload());
            let (mut ckb_cells, mut out_points) =
                self.collect_inputs_for_asset_collection_ckb(address.clone())?;
            sigs_entry.push(SignatureEntry {
                type_: WitnessType::WitnessArgsLock,
                index: all_out_points.len(),
                group_len: out_points.len(),
                pub_key: address.to_string(),
                sig_type: SignatureType::Secp256k1,
            });
            all_out_points.append(&mut out_points);
            all_ckb_cells.append(&mut ckb_cells);
        }
        let ckb_consumed = all_ckb_cells
            .iter()
            .map(|cell| {
                let capacity: u64 = cell.cell_output.capacity().unpack();
                capacity
            })
            .sum::<u64>();
        Ok((ckb_consumed, all_out_points, sigs_entry))
    }

    fn collect_inputs_for_asset_collection_ckb(
        &self,
        addr: Address,
    ) -> Result<(Vec<DetailedLiveCell>, Vec<packed::OutPoint>)> {
        let tx_pool = read_tx_pool_cache();
        let script = address_to_script(addr.payload());
        let mut out_points = vec![];
        let mut detailed_live_cells = vec![];
        let cells = self.get_cells_by_lock_script(&script)?;
        for (ckb_cell, out_point) in ckb_iter(&cells) {
            if tx_pool.contains(&out_point) {
                continue;
            }
            out_points.push(out_point.to_owned());
            detailed_live_cells.push(ckb_cell.to_owned());
        }
        Ok((detailed_live_cells, out_points))
    }

    fn collect_claimable_cells_for_asset_collection_udt(
        &self,
        addr: Address,
        udt_hash: H256,
    ) -> Result<Vec<DetailedCell>> {
        let tx_pool = read_tx_pool_cache();
        let script = address_to_script(addr.payload());
        let sp_cells = self.get_sp_cells_by_addr(&addr)?;
        let receiver_lock_hash = blake2b_160(script.as_slice());
        let config = self.get_config(special_cells::CHEQUE)?;
        let current_epoch = {
            let epoch = CURRENT_EPOCH.read();
            epoch.clone()
        };
        let claimable_cells = sp_cells
            .0
            .iter()
            .filter(|cell| !tx_pool.contains(&cell.out_point))
            .filter(|cell| {
                // filter CHEQUE cell
                cell.cell_output.lock().code_hash() == config.code_hash()
                    && cell.cell_output.lock().hash_type() == config.hash_type()
            })
            .filter(|cell| {
                // filter receiver lock_hash
                let lock_args = cell.cell_output.lock().args().raw_data();
                lock_args.len() == 40 && lock_args[0..20] == receiver_lock_hash
            })
            .filter(move |cell| {
                // filter claimable cell
                let cell_epoch = RationalU256::from_u256(cell.epoch_number.clone());
                let cheque_since = self.cheque_since.clone();
                current_epoch.clone().sub(cell_epoch) < cheque_since
            })
            .filter(|cell| {
                // filter out udt type script
                let type_script_opt = cell.cell_output.type_().to_opt();
                type_script_opt.is_some()
                    && type_script_opt.unwrap().calc_script_hash() == udt_hash.pack()
            })
            .map(|cell| cell.to_owned())
            .collect::<Vec<_>>();

        Ok(claimable_cells)
    }

    fn build_output_for_asset_collection(
        &self,
        to: Address,
        amount: u64,
        udt_hash: Option<H256>,
    ) -> packed::CellOutput {
        let lock_script = address_to_script(to.payload());
        packed::CellOutputBuilder::default()
            .lock(lock_script)
            .capacity(amount.pack())
            .build()
    }

    // For simplicity, it required the pay fee address must hash enough capacity in at least on lived cell
    // to pay fee and generate a change cell.
    fn pay_fee(
        &self,
        fee_address: Address,
        fee: u64,
    ) -> Result<(packed::OutPoint, packed::CellOutput)> {
        let tx_pool = read_tx_pool_cache();
        let lock_script = address_to_script(fee_address.payload());
        let cells = self.get_cells_by_lock_script(&lock_script)?;
        for (ckb_cell, out_point) in ckb_iter(&cells) {
            if tx_pool.contains(&out_point) {
                continue;
            }
            let ckb_capacity: u64 = ckb_cell.cell_output.capacity().unpack();
            if ckb_capacity < fee + MIN_CKB_CAPACITY {
                continue;
            }
            let change = ckb_capacity - fee;
            let change_cell = packed::CellOutputBuilder::default()
                .lock(lock_script)
                .capacity(change.pack())
                .build();
            return Ok((out_point.to_owned(), change_cell));
        }
        Err(MercuryError::rpc(RpcError::FeePaiedByAddressInsufficientCapacity).into())
    }

    // fn inner_collect_asset_udt(&self) -> Result<TransactionCompletionResponse> {}

    fn build_tx_view(
        &self,
        deps: Vec<packed::CellDep>,
        inputs: Vec<packed::OutPoint>,
        outputs: Vec<packed::CellOutput>,
        data: Vec<packed::Bytes>,
    ) -> TransactionView {
        let since: packed::Uint64 = 0u64.pack();

        TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .cell_deps(deps)
            .inputs(inputs.into_iter().map(|input| {
                packed::CellInputBuilder::default()
                    .since(since.clone())
                    .previous_output(input)
                    .build()
            }))
            .outputs(outputs)
            .outputs_data(data)
            .build()
    }

    fn build_inputs(
        &self,
        udt_hash: &Option<H256>,
        from: InnerAccount,
        amounts: &DetailedAmount,
        fee: u64,
        inputs: &mut Vec<packed::OutPoint>,
        outputs: &mut Vec<packed::CellOutput>,
        outputs_data: &mut Vec<packed::Bytes>,
        script_set: &mut HashSet<String>,
    ) -> Result<(InputConsume, Vec<SignatureEntry>)> {
        let mut ckb_needed = if udt_hash.is_some() {
            if amounts.ckb_all == 0 {
                BigUint::from(fee + MIN_CKB_CAPACITY)
            } else {
                BigUint::from(amounts.ckb_all + fee + MIN_CKB_CAPACITY + STANDARD_SUDT_CAPACITY)
            }
        } else {
            BigUint::from(amounts.ckb_all + fee + MIN_CKB_CAPACITY)
        };
        let mut udt_needed = BigUint::from(amounts.udt_amount);
        let (mut capacity_sum, mut udt_sum) = (0u64, 0u128);
        let (mut sigs_entry, mut cheque_sigs_entry, mut acp_sigs_entry) =
            (HashMap::new(), vec![], vec![]);

        // Todo: can refactor here.
        if udt_needed.is_zero() {
            // An CkB transfer transaction.
            for ident in from.idents.iter() {
                let addr = parse_address(ident)?;
                let script = address_to_script(addr.payload());
                let cells = self.get_cells_by_lock_script(&script)?;
                let ckb_iter = ckb_iter(&cells);

                self.pool_ckb(
                    ckb_iter,
                    &mut ckb_needed,
                    inputs,
                    &mut sigs_entry,
                    &mut capacity_sum,
                );
            }

            if ckb_needed > Zero::zero() {
                return Err(
                    MercuryError::rpc(RpcError::CkbIsNotEnough(from.idents[0].clone())).into(),
                );
            }
        } else {
            // An UDT transfer transaction.
            let udt_hash = udt_hash.clone().unwrap();

            for ident in from.idents.iter() {
                let addr = parse_address(ident)?;
                let script = address_to_script(addr.payload());
                let cells = self.get_cells_by_lock_script(&script)?;
                let ckb_iter = ckb_iter(&cells);
                let udt_iter = udt_iter(&cells, udt_hash.pack());
                let sp_cells = self.get_sp_cells_by_addr(&addr)?.inner();
                let acps_by_from = self.take_sp_cells(&sp_cells, special_cells::ACP)?;

                if from.scripts.contains(&ScriptType::ClaimableCheque) {
                    script_set.insert(ScriptType::Secp256k1.as_str().to_string());
                    self.pool_claimable_cheque(
                        addr.payload(),
                        sp_cells,
                        &mut udt_needed,
                        inputs,
                        outputs,
                        outputs_data,
                        &mut udt_sum,
                        &mut cheque_sigs_entry,
                    )?;
                } else {
                    // Pool for UDT.
                    self.pool_udt(
                        udt_iter,
                        &mut udt_needed,
                        inputs,
                        &mut capacity_sum,
                        &mut udt_sum,
                        &mut sigs_entry,
                    );

                    self.pool_udt_acp(
                        &udt_hash,
                        &addr,
                        &acps_by_from,
                        &mut udt_needed,
                        inputs,
                        outputs,
                        outputs_data,
                        &mut acp_sigs_entry,
                    )?;
                }

                // Pool for ckb of UDT capacity.
                self.pool_ckb(
                    ckb_iter,
                    &mut ckb_needed,
                    inputs,
                    &mut sigs_entry,
                    &mut capacity_sum,
                );
            }

            if udt_needed > Zero::zero() {
                return Err(
                    MercuryError::rpc(RpcError::UDTIsNotEnough(from.idents[0].clone())).into(),
                );
            }

            if ckb_needed > Zero::zero() {
                return Err(
                    MercuryError::rpc(RpcError::CkbIsNotEnough(from.idents[0].clone())).into(),
                );
            }

            if let Some((_id, mut acp_cells)) = (*ACP_USED_CACHE).remove(&thread::current().id()) {
                inputs.append(&mut acp_cells);
            }
        }

        let mut sigs_entry = sigs_entry.into_iter().map(|(_k, v)| v).collect::<Vec<_>>();
        sigs_entry.append(&mut acp_sigs_entry);
        sigs_entry.append(&mut cheque_sigs_entry);
        sigs_entry.sort();

        Ok((InputConsume::new(capacity_sum, udt_sum), sigs_entry))
    }

    fn pool_claimable_cheque(
        &self,
        addr: &AddressPayload,
        sp_cells: Vec<DetailedCell>,
        udt_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        outputs: &mut Vec<packed::CellOutput>,
        outputs_data: &mut Vec<packed::Bytes>,
        udt_sum: &mut u128,
        sigs_entry: &mut Vec<SignatureEntry>,
    ) -> Result<()> {
        let tx_pool = read_tx_pool_cache();
        let lock_hash = blake2b_160(address_to_script(addr).as_slice());

        for cell in self
            .take_cheque_cells(&sp_cells, &lock_hash, true)?
            .into_iter()
        {
            if udt_needed.is_zero() {
                break;
            }

            if self.is_cheque_cell_outdated(&cell) || tx_pool.contains(&cell.out_point) {
                continue;
            }

            let lock_args: Vec<u8> = cell.cell_output.lock().args().unpack();
            let amount = decode_udt_amount(&cell.cell_data.raw_data().to_vec());
            let udt_used = amount.min(udt_needed.clone().try_into().unwrap());
            inputs.push(cell.out_point.clone());

            // Build CKB cell for sender.
            let mut sender_hash = [0u8; 20];
            sender_hash.copy_from_slice(&lock_args[20..40]);
            let sender_lock_script = self.get_script_by_hash(sender_hash)?;
            outputs.push(
                packed::CellOutputBuilder::default()
                    .lock(sender_lock_script)
                    .capacity(cell.cell_output.capacity())
                    .build(),
            );
            outputs_data.push(packed::Bytes::default());

            *udt_needed -= udt_used;
            *udt_sum += amount;

            let addr = Address::new(self.net_ty, addr.clone()).to_string();
            sigs_entry.push(SignatureEntry::new(
                inputs.len() - 1,
                addr,
                SignatureType::Secp256k1,
            ));
        }

        Ok(())
    }

    fn build_outputs(
        &self,
        udt_hash: &Option<H256>,
        to_addr: &Address,
        ckb_amount: u64,
        udt_amount: u128,
        script: &ScriptType,
        amounts: &mut DetailedAmount,
        from_addr: String,
    ) -> Result<CellWithData> {
        if script.is_acp() {
            return self.build_acp_outputs(udt_hash, to_addr, udt_amount, amounts);
        }

        if script.is_my_acp() {
            return self.build_my_acp_outputs(udt_hash, to_addr, udt_amount, amounts);
        }

        let (type_script, data) = self.build_type_script(udt_hash.clone(), udt_amount)?;
        let lock_script = self.build_lock_script(to_addr.payload(), script, from_addr)?;
        let capacity = if udt_hash.is_none() {
            let max = (ckb_amount * BYTE_SHANNONS).max(MIN_CKB_CAPACITY);
            amounts.add_ckb_all(max);
            max
        } else {
            amounts.add_udt_amount(udt_amount);

            if script.is_cheque() {
                amounts.add_ckb_all(CHEQUE_CELL_CAPACITY);
                CHEQUE_CELL_CAPACITY
            } else {
                amounts.add_ckb_all(STANDARD_SUDT_CAPACITY);
                STANDARD_SUDT_CAPACITY
            }
        };

        let cell = packed::CellOutputBuilder::default()
            .lock(lock_script)
            .type_(type_script.pack())
            .capacity(capacity.pack())
            .build();

        Ok(CellWithData::new(cell, data))
    }

    fn build_type_script(
        &self,
        udt_hash: Option<H256>,
        amount: u128,
    ) -> Result<(Option<packed::Script>, Bytes)> {
        if let Some(hash) = udt_hash {
            let byte32 = hash.pack();
            let key = udt_balance::Key::ScriptHash(&byte32);
            let mut script_bytes = self
                .store_get(*UDT_EXT_PREFIX, key.into_vec())?
                .ok_or_else(|| {
                    MercuryError::rpc(RpcError::UDTInexistence(hex::encode(hash.as_bytes())))
                })?;
            let _is_sudt = script_bytes.remove(0) == 1;
            let script = packed::Script::from_slice(&script_bytes).unwrap();
            let data = Bytes::from(amount.to_le_bytes().to_vec());

            Ok((Some(script), data))
        } else {
            Ok((None, Default::default()))
        }
    }

    fn build_lock_script(
        &self,
        to_addr: &AddressPayload,
        script: &ScriptType,
        from_addr: String,
    ) -> Result<packed::Script> {
        let script_builder = packed::ScriptBuilder::default();

        let script = match script {
            ScriptType::Secp256k1 => address_to_script(to_addr),
            ScriptType::Cheque => {
                let code_hash = self
                    .config
                    .get(CHEQUE)
                    .ok_or_else(|| MercuryError::rpc(RpcError::MissingConfig(CHEQUE.to_string())))?
                    .script
                    .code_hash();
                let receiver_lock = address_to_script(&to_addr);
                let sender_lock = address_to_script(parse_address(&from_addr)?.payload());
                let mut lock_args = Vec::from(blake2b_160(receiver_lock.as_slice()));
                lock_args.extend_from_slice(&blake2b_160(sender_lock.as_slice()));

                script_builder
                    .code_hash(code_hash)
                    .hash_type(ScriptHashType::Type.into())
                    .args(lock_args.pack())
                    .build()
            }
            _ => unreachable!(),
        };

        Ok(script)
    }

    // This function is called when to_action is PayByFrom and udt_hash is some
    fn build_acp_outputs(
        &self,
        udt_hash: &Option<H256>,
        to_addr: &Address,
        amount: u128,
        amounts: &mut DetailedAmount,
    ) -> Result<CellWithData> {
        let (udt_script, data) = self.build_type_script(udt_hash.clone(), amount)?;
        let capacity = STANDARD_SUDT_CAPACITY;
        let lock_args = self.build_acp_lock_args(to_addr.payload().args(), None, None)?;
        let acp_lock = self
            .config
            .get(special_cells::ACP)
            .ok_or_else(|| {
                MercuryError::rpc(RpcError::MissingConfig(special_cells::ACP.to_string()))
            })?
            .clone()
            .script;
        let cell = packed::CellOutputBuilder::default()
            .type_(udt_script.pack())
            .lock(acp_lock.as_builder().args(lock_args.pack()).build())
            .capacity(capacity.pack())
            .build();

        amounts.add_udt_amount(amount);
        amounts.add_ckb_all(capacity);

        Ok(CellWithData::new(cell, data))
    }

    // This function is called when to_action is PayByTo
    fn build_my_acp_outputs(
        &self,
        udt_hash: &Option<H256>,
        to_addr: &Address,
        amount: u128,
        amounts: &mut DetailedAmount,
    ) -> Result<CellWithData> {
        // Find an ACP cell with the given sudt hash.
        let sudt_hash = udt_hash.clone().unwrap();
        let sp_cells = self.get_sp_cells_by_addr(to_addr)?.inner();
        let acp_cells = self.take_sp_cells(&sp_cells, special_cells::ACP)?;
        let mut acp_cell = acp_cells
            .iter()
            .find(|cell| {
                cell.cell_output.type_().is_some()
                    && cell
                        .cell_output
                        .type_()
                        .to_opt()
                        .unwrap()
                        .calc_script_hash()
                        == sudt_hash.pack()
            })
            .cloned()
            .ok_or_else(|| {
                MercuryError::rpc(RpcError::MissingACPCell(
                    to_addr.to_string(),
                    hex::encode(sudt_hash.as_ref()),
                ))
            })?;

        let sudt_amount = decode_udt_amount(&acp_cell.cell_data.raw_data());
        let new_sudt_amount = sudt_amount + amount;
        acp_cell.cell_data = new_sudt_amount.to_le_bytes().to_vec().pack();
        amounts.add_udt_amount(amount);

        // Add ACP used to the cache.
        ACP_USED_CACHE
            .entry(thread::current().id())
            .or_insert_with(Vec::new)
            .push(acp_cell.out_point);

        Ok(CellWithData::new(
            acp_cell.cell_output,
            acp_cell.cell_data.unpack(),
        ))
    }

    fn build_change_cell(
        &self,
        addr: String,
        udt_hash: Option<H256>,
        ckb_change: u64,
        udt_change: u128,
    ) -> Result<(Vec<packed::CellOutput>, Vec<packed::Bytes>)> {
        let address = parse_address(&addr)?;
        let (mut ret_cell, mut ret_data) = (vec![], vec![]);
        let (type_script, data) = self.build_type_script(udt_hash, udt_change)?;
        let lock_script = self.build_lock_script(
            address.payload(),
            &ScriptType::Secp256k1,
            Default::default(),
        )?;
        let ckb_capacity = if udt_change != 0 {
            ckb_change - STANDARD_SUDT_CAPACITY
        } else {
            ckb_change
        };

        if type_script.is_some() && udt_change != 0 {
            ret_cell.push(
                packed::CellOutputBuilder::default()
                    .type_(type_script.pack())
                    .lock(lock_script.clone())
                    .capacity(STANDARD_SUDT_CAPACITY.pack())
                    .build(),
            );
            ret_data.push(data.pack());
        }

        if ckb_capacity != 0 {
            ret_cell.push(
                packed::CellOutputBuilder::default()
                    .lock(lock_script)
                    .capacity(ckb_capacity.pack())
                    .build(),
            );
            ret_data.push(Default::default());
        }

        Ok((ret_cell, ret_data))
    }

    fn pool_udt_acp(
        &self,
        udt_hash: &H256,
        from: &Address,
        acp_cells: &[DetailedCell],
        sudt_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        outputs: &mut Vec<packed::CellOutput>,
        outputs_data: &mut Vec<packed::Bytes>,
        sigs_entry: &mut Vec<SignatureEntry>,
    ) -> Result<()> {
        let tx_pool = read_tx_pool_cache();

        for detail in acp_cells.iter() {
            if sudt_needed.is_zero() {
                break;
            }

            if tx_pool.contains(&detail.out_point) {
                continue;
            }

            if let Some(type_script) = detail.cell_output.type_().to_opt() {
                if type_script.calc_script_hash() != udt_hash.pack() {
                    continue;
                }

                let acp_data = detail.cell_data.raw_data();
                let sudt_amount = decode_udt_amount(&acp_data);
                let new_sudt_amount = u128_sub(sudt_amount, sudt_needed.clone());
                let mut new_cell_data = encode_udt_amount(new_sudt_amount);
                new_cell_data.extend_from_slice(&acp_data[16..]);

                inputs.push(detail.out_point.clone());
                outputs.push(detail.cell_output.clone());
                outputs_data.push(new_cell_data.pack());

                *sudt_needed -= sudt_amount.min(sudt_needed.clone().try_into().unwrap());

                sigs_entry.push(SignatureEntry::new(
                    inputs.len() - 1,
                    from.display_with_network(self.net_ty),
                    SignatureType::Secp256k1,
                ));
            }
        }

        Ok(())
    }

    fn pool_ckb<'a, I: Iterator<Item = &'a (DetailedLiveCell, packed::OutPoint)>>(
        &self,
        ckb_iter: I,
        ckb_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        sigs_entry: &mut HashMap<String, SignatureEntry>,
        capacity_sum: &mut u64,
    ) {
        let tx_pool = read_tx_pool_cache();

        for (ckb_cell, out_point) in ckb_iter {
            if ckb_needed.is_zero() {
                break;
            }

            if tx_pool.contains(&out_point) {
                continue;
            }

            let capacity: u64 = ckb_cell.cell_output.capacity().unpack();
            let consume_ckb = capacity.min(ckb_needed.clone().try_into().unwrap());
            inputs.push(out_point.clone());

            *ckb_needed -= consume_ckb;
            *capacity_sum += capacity;

            let addr = Address::new(self.net_ty, ckb_cell.cell_output.lock().into()).to_string();
            if let Some(entry) = sigs_entry.get_mut(&addr) {
                entry.add_group();
            } else {
                sigs_entry.insert(
                    addr.clone(),
                    SignatureEntry::new(inputs.len() - 1, addr, SignatureType::Secp256k1),
                );
            }
        }
    }

    fn pool_udt<'a, I: Iterator<Item = &'a (DetailedLiveCell, packed::OutPoint)>>(
        &self,
        udt_iter: I,
        udt_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        capacity_sum: &mut u64,
        udt_sum: &mut u128,
        sigs_entry: &mut HashMap<String, SignatureEntry>,
    ) {
        let tx_pool = read_tx_pool_cache();

        for (udt_cell, out_point) in udt_iter {
            if udt_needed.is_zero() {
                break;
            }

            if tx_pool.contains(&out_point) {
                continue;
            }

            let capacity: u64 = udt_cell.cell_output.capacity().unpack();
            let amount = decode_udt_amount(&udt_cell.cell_data.raw_data().to_vec());
            let udt_used = amount.min(udt_needed.clone().try_into().unwrap());
            inputs.push(out_point.clone());

            *udt_needed -= udt_used;
            *udt_sum += amount;
            *capacity_sum += capacity;

            let addr = Address::new(self.net_ty, udt_cell.cell_output.lock().into()).to_string();
            if let Some(entry) = sigs_entry.get_mut(&addr) {
                entry.add_group();
            } else {
                sigs_entry.insert(
                    addr.clone(),
                    SignatureEntry::new(inputs.len() - 1, addr, SignatureType::Secp256k1),
                );
            }
        }
    }

    fn build_cell_deps(&self, scripts_set: HashSet<String>) -> Vec<packed::CellDep> {
        scripts_set
            .into_iter()
            .map(|s| self.config.get(s.as_str()).cloned().unwrap().cell_dep)
            .collect()
    }

    fn take_sp_cells(
        &self,
        cell_list: &[DetailedCell],
        cell_name: &str,
    ) -> Result<Vec<DetailedCell>> {
        let script_code_hash = self
            .config
            .get(cell_name)
            .ok_or_else(|| MercuryError::rpc(RpcError::MissingConfig(cell_name.to_string())))?
            .script
            .code_hash();

        Ok(cell_list
            .iter()
            .filter(|cell| cell.cell_output.lock().code_hash() == script_code_hash)
            .cloned()
            .collect())
    }

    fn take_cheque_cells(
        &self,
        cell_list: &[DetailedCell],
        lock_hash: &[u8],
        is_receiver: bool,
    ) -> Result<Vec<DetailedCell>> {
        let script_code_hash = self
            .config
            .get("cheque")
            .ok_or_else(|| MercuryError::rpc(RpcError::MissingConfig("cheque".to_string())))?
            .script
            .code_hash();
        let iter = cell_list
            .iter()
            .filter(|cell| cell.cell_output.lock().code_hash() == script_code_hash);

        let ret = if is_receiver {
            iter.filter(|cell| {
                let args: Vec<u8> = cell.cell_output.lock().args().unpack();
                &args[0..20] == lock_hash
            })
            .cloned()
            .collect::<Vec<_>>()
        } else {
            iter.filter(|cell| {
                let args: Vec<u8> = cell.cell_output.lock().args().unpack();
                &args[20..40] == lock_hash
            })
            .cloned()
            .collect::<Vec<_>>()
        };

        Ok(ret)
    }

    fn build_acp_lock_args(
        &self,
        pubkey_hash: Bytes,
        ckb_min: Option<u8>,
        udt_min: Option<u8>,
    ) -> Result<Bytes> {
        let mut ret = pubkey_hash.to_vec();
        if let Some(min) = ckb_min {
            ret.push(min);
        }

        if let Some(min) = udt_min {
            ret.push(min);
        }

        Ok(ret.into())
    }

    fn is_cheque_cell_outdated(&self, cell: &DetailedCell) -> bool {
        let epoch = cell.epoch_number.clone();
        let current_epoch = CURRENT_EPOCH.read().clone();
        (current_epoch - epoch) > self.cheque_since
    }

    fn update_tx_view_change_cell(
        &self,
        tx_view: JsonTransactionView,
        change_address: Address,
        estimate_fee: u64,
        actual_fee: u64,
    ) -> Result<JsonTransactionView> {
        let mut tx = tx_view.inner;
        let change_cell_lock = self.build_lock_script(
            &change_address.payload(),
            &ScriptType::Secp256k1,
            Default::default(),
        )?;
        for output in &mut tx.outputs {
            if output.lock == change_cell_lock.clone().into() && output.type_.is_none() {
                let change_cell_capacity: u64 = output.capacity.into();
                let updated_change_cell_capacity = change_cell_capacity + estimate_fee - actual_fee;
                let updated_change_cell = packed::CellOutputBuilder::default()
                    .lock(change_cell_lock)
                    .capacity(updated_change_cell_capacity.pack())
                    .build();
                *output = updated_change_cell.into();
                let raw_updated_tx = packed::Transaction::from(tx).raw();
                let updated_tx_view = TransactionBuilder::default()
                    .version(TX_VERSION.pack())
                    .cell_deps(raw_updated_tx.cell_deps())
                    .inputs(raw_updated_tx.inputs())
                    .outputs(raw_updated_tx.outputs())
                    .outputs_data(raw_updated_tx.outputs_data())
                    .build();
                return Ok(updated_tx_view.into());
            }
        }
        Err(MercuryError::rpc(RpcError::CannotFindChangeCell).into())
    }
}

fn read_tx_pool_cache() -> HashSet<packed::OutPoint> {
    let cache = TX_POOL_CACHE.read();
    cache.clone()
}

fn calculate_tx_size_with_witness_placeholder(
    tx_view: JsonTransactionView,
    sigs_entry: Vec<SignatureEntry>,
) -> usize {
    let tx = tx_view.inner;
    let raw_tx = packed::Transaction::from(tx.clone()).raw();
    let mut witnesses_map = HashMap::new();
    for (index, _input) in tx.inputs.into_iter().enumerate() {
        witnesses_map.insert(index, Bytes::new());
    }
    for sig_entry in sigs_entry {
        let witness = packed::WitnessArgs::new_builder()
            .lock(Some(Bytes::from(vec![0u8; 65])).pack())
            .build();
        witnesses_map.insert(sig_entry.index, witness.as_bytes());
    }

    let witnesses: Vec<packed::Bytes> = witnesses_map
        .into_iter()
        .map(|(_index, witness)| witness.pack())
        .collect();

    let tx_view_with_witness_placeholder = TransactionBuilder::default()
        .version(TX_VERSION.pack())
        .cell_deps(raw_tx.cell_deps())
        .inputs(raw_tx.inputs())
        .outputs(raw_tx.outputs())
        .outputs_data(raw_tx.outputs_data())
        .witnesses(witnesses)
        .build();
    let tx_size = tx_view_with_witness_placeholder.data().total_size();
    // tx offset bytesize
    tx_size + 4
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_address_to_pubkey() {
        let addr_1 = parse_address("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v").unwrap();
        let addr_2 = parse_address("ckb1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3vumhs9nvu786dj9p0q5elx66t24n3kxgj53qks").unwrap();

        assert_eq!(addr_1.payload().args(), addr_2.payload().args());
        assert_eq!(addr_1.payload().args().len(), 20);

        let lock_script = address_to_script(addr_1.payload());
        assert_eq!(lock_script.args().raw_data(), addr_2.payload().args());
    }
}
