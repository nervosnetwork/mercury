use crate::error::MercuryError;
use crate::extensions::{special_cells, udt_balance, DetailedCell, CURRENT_EPOCH, UDT_EXT_PREFIX};
use crate::rpc::rpc_impl::{
    address_to_script, capacity_detail, ckb_iter, udt_iter, MercuryRpcImpl, ACP_USED_CACHE,
    BYTE_SHANNONS, MIN_CKB_CAPACITY,
};
use crate::rpc::types::{
    details_split_off, CellWithData, DetailedAmount, InnerAccount, InnerTransferItem, InputConsume,
    ScriptType, SignatureEntry, TransferCompletionResponse, WitnessType, CHEQUE,
};
use crate::utils::{
    decode_udt_amount, encode_udt_amount, parse_address, u128_sub, u64_sub, unwrap_only_one,
};

use anyhow::Result;
use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_sdk::Address;
use ckb_types::core::{ScriptHashType, TransactionBuilder, TransactionView};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H160, H256};
use num_bigint::BigUint;
use num_traits::identities::Zero;

use std::{collections::HashSet, convert::TryInto, iter::Iterator, str::FromStr, thread};

impl<S: Store> MercuryRpcImpl<S> {
    pub(crate) fn inner_transfer_complete(
        &self,
        udt_hash: Option<H256>,
        from: InnerAccount,
        items: Vec<InnerTransferItem>,
        change: Option<String>,
        fee: u64,
    ) -> Result<TransferCompletionResponse> {
        let mut amounts = DetailedAmount::new();
        let amount = items.iter().map(|i| i.amount).sum();
        let mut output_capacity = 0u64;
        let mut scripts_set = from.scripts.clone().into_iter().collect::<HashSet<_>>();
        let (mut inputs, mut sigs_entry) = (vec![], vec![]);
        let (mut outputs, mut cell_data) = (vec![], vec![]);
        let change = change.unwrap_or_else(|| from.idents[0].clone());

        if from.scripts.contains(&ScriptType::Cheque) {
            self.build_cheque_claim(
                &udt_hash,
                &from,
                amount,
                fee,
                &mut inputs,
                &mut sigs_entry,
                &mut outputs,
                &mut cell_data,
            )?;
        }

        for item in items.iter() {
            let addr = unwrap_only_one(&item.to.idents);
            let script = unwrap_only_one(&item.to.scripts);
            scripts_set.insert(script);
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
                &mut output_capacity,
            )?;

            details_split_off(output_cells, &mut outputs, &mut cell_data);
        }

        let consume = self.build_inputs(
            &udt_hash,
            from,
            &amounts,
            fee,
            &mut inputs,
            &mut sigs_entry,
            &mut outputs,
            &mut cell_data,
        )?;
        let (change_cell, change_data) = self.build_change_cell(
            change,
            udt_hash,
            output_capacity - consume.ckb - fee,
            amounts.udt_amount - consume.udt,
        )?;
        let cell_deps = self.build_cell_deps(scripts_set);

        outputs.push(change_cell);
        cell_data.push(change_data);

        let view = self.build_tx_view(cell_deps, inputs, outputs, cell_data);
        let tx_hash = view.hash().raw_data();
        sigs_entry
            .iter_mut()
            .for_each(|entry| entry.message = tx_hash.clone());

        Ok(TransferCompletionResponse::new(view.into(), sigs_entry))
    }

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
        sigs_entry: &mut Vec<SignatureEntry>,
        outputs: &mut Vec<packed::CellOutput>,
        output_data: &mut Vec<packed::Bytes>,
    ) -> Result<InputConsume> {
        let mut ckb_needed = BigUint::from(amounts.ckb_all + fee + MIN_CKB_CAPACITY);
        let mut udt_needed = BigUint::from(amounts.udt_amount);
        let (mut acp_outputs, mut capacity_sum, mut udt_sum_except_acp) = (vec![], 0u64, 0u128);

        // Todo: can refactor here.
        if udt_needed.is_zero() {
            // An CkB transfer transaction.
            for ident in from.idents.iter() {
                let addr = Address::from_str(ident).map_err(MercuryError::ParseCKBAddressError)?;
                let script = address_to_script(addr.payload());
                let (cells, out_points) = self.get_cells_by_lock_script(&script)?;
                let sp_cells = self.get_sp_cells_by_addr(&addr)?.inner();
                let acps_by_from = self.take_sp_cells(&sp_cells, special_cells::ACP);
                let ckb_iter = ckb_iter(&cells, &out_points);

                self.pool_acp(
                    acps_by_from,
                    &mut ckb_needed,
                    &mut udt_needed,
                    inputs,
                    &mut acp_outputs,
                    &mut capacity_sum,
                )?;

                self.pool_ckb(
                    ckb_iter,
                    &mut ckb_needed,
                    inputs,
                    sigs_entry,
                    &mut capacity_sum,
                );
            }
        } else {
            // An UDT transfer transaction.
            let udt_hash = udt_hash.clone().unwrap();

            for ident in from.idents.iter() {
                let addr = parse_address(ident)?;
                let script = address_to_script(addr.payload());
                let (cells, out_points) = self.get_cells_by_lock_script(&script)?;
                let ckb_iter = ckb_iter(&cells, &out_points);
                let udt_iter = udt_iter(&cells, &out_points, udt_hash.pack());
                let sp_cells = self.get_sp_cells_by_addr(&addr)?.inner();
                let acps_by_from = self.take_sp_cells(&sp_cells, special_cells::ACP);

                self.pool_acp(
                    acps_by_from,
                    &mut ckb_needed,
                    &mut udt_needed,
                    inputs,
                    &mut acp_outputs,
                    &mut capacity_sum,
                )?;

                self.pool_udt(
                    udt_iter,
                    &mut udt_needed,
                    inputs,
                    &mut capacity_sum,
                    &mut udt_sum_except_acp,
                );

                self.pool_ckb(
                    ckb_iter,
                    &mut ckb_needed,
                    inputs,
                    sigs_entry,
                    &mut capacity_sum,
                );
            }

            details_split_off(acp_outputs, outputs, output_data);

            // Todo: can do perf here.
            if let Some(tmp) = (*ACP_USED_CACHE).get(&thread::current().id()) {
                let (mut acp_used, ckb_used) = tmp.clone();
                inputs.append(&mut acp_used);
                capacity_sum += ckb_used;
            }
        }

        Ok(InputConsume::new(capacity_sum, udt_sum_except_acp))
    }

    fn build_cheque_claim(
        &self,
        udt_hash: &Option<H256>,
        from: &InnerAccount,
        mut amount: u128,
        fee: u64,
        _inputs: &mut Vec<packed::OutPoint>,
        _sigs_entry: &mut Vec<SignatureEntry>,
        _outputs: &mut Vec<packed::CellOutput>,
        _output_data: &mut Vec<packed::Bytes>,
    ) -> Result<()> {
        let mut cheque_claim = Vec::new();

        if let Some(hash) = udt_hash {
            for ident in from.idents.iter() {
                let addr = parse_address(ident)?;
                let cells = self.take_cheque_claimable_cell(&addr, hash.pack())?;
                let udt_amounts = cells
                    .iter()
                    .map(|cell| decode_udt_amount(&cell.cell_data.raw_data()))
                    .collect::<Vec<_>>();

                for (cell, udt_amount) in cells.iter().zip(udt_amounts.iter()) {
                    if amount == 0 {
                        break;
                    }

                    let min = *udt_amount.min(&amount);
                    cheque_claim.push(cell.clone());
                    amount -= min;
                }
            }

            for cell in cheque_claim.iter() {
                // let mut tx = (vec![], vec![]);
                let script = cell.cell_output.lock();
                let (cells, out_points) = self.get_cells_by_lock_script(&script)?;
                let sudt_cell = udt_iter(&cells, &out_points, hash.pack()).next();

                if sudt_cell.is_none() {
                    return Err(MercuryError::LackSUDTCells(hex::encode(
                        cell.cell_output.lock().calc_script_hash().raw_data(),
                    ))
                    .into());
                }

                let _sudt_cell = sudt_cell.unwrap();

                todo!()
            }
        } else {
            let mut amount = fee + amount as u64;
            for ident in from.idents.iter() {
                let addr = parse_address(ident)?;
                let cells = self.take_cheque_redeemable_cell(&addr)?;
                let capacity_vec = cells
                    .iter()
                    .map(|cell| cell.cell_output.capacity().unpack())
                    .collect::<Vec<u64>>();

                for (cell, ckb) in cells.iter().zip(capacity_vec.iter()) {
                    if amount == 0 {
                        break;
                    }

                    let min = *ckb.min(&amount);
                    cheque_claim.push(cell.clone());
                    amount -= min;
                }
            }

            todo!();
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
        capacity_sum: &mut u64,
    ) -> Result<Vec<CellWithData>> {
        if script.is_acp() {
            return self.build_acp_outputs(
                udt_hash,
                to_addr,
                from_addr,
                udt_amount,
                amounts,
                capacity_sum,
            );
        }

        let (type_script, data) = self.build_type_script(udt_hash.clone(), udt_amount, amounts)?;
        let lock_script = self.build_lock_script(to_addr, script, from_addr)?;
        let cell = packed::CellOutputBuilder::default()
            .lock(lock_script)
            .type_(type_script.pack())
            .build();
        let mut capacity: u64 = cell.capacity().unpack();

        if udt_hash.is_none() {
            capacity += ckb_amount;
            amounts.add_ckb_all(ckb_amount);
        } else {
            capacity += (data.len() as u64) * BYTE_SHANNONS;
            self.add_detailed_amount(amounts, to_addr.to_string(), capacity, script);
        }

        *capacity_sum += capacity;

        Ok(vec![CellWithData::new(
            cell.as_builder().capacity(capacity.pack()).build(),
            data,
        )])
    }

    fn build_type_script(
        &self,
        udt_hash: Option<H256>,
        amount: u128,
        amounts: &mut DetailedAmount,
    ) -> Result<(Option<packed::Script>, Bytes)> {
        if let Some(hash) = udt_hash {
            let byte32 = hash.pack();
            let key = udt_balance::Key::ScriptHash(&byte32);
            let mut script_bytes = self
                .store_get(*UDT_EXT_PREFIX, key.into_vec())?
                .ok_or_else(|| MercuryError::UDTInexistence(hex::encode(hash.as_bytes())))?;
            let _is_sudt = script_bytes.remove(0) == 1;
            let script = packed::Script::from_slice(&script_bytes).unwrap();
            let data = Bytes::from(amount.to_le_bytes().to_vec());
            amounts.add_udt_amount(amount);

            Ok((Some(script), data))
        } else {
            Ok((None, Default::default()))
        }
    }

    fn build_lock_script(
        &self,
        to_addr: &Address,
        script: &ScriptType,
        from_addr: String,
    ) -> Result<packed::Script> {
        let script_builder = packed::ScriptBuilder::default();

        let script: packed::Script = match script {
            ScriptType::Secp256k1 => to_addr.payload().into(),
            ScriptType::Cheque => {
                let code_hash = self.config.get(CHEQUE).unwrap().script.code_hash();
                let receiver_lock: packed::Script = to_addr.payload().into();
                let sender_lock: packed::Script = parse_address(&from_addr)?.payload().into();
                let mut lock_args = Vec::from(&receiver_lock.calc_script_hash().as_slice()[0..20]);
                lock_args.extend_from_slice(&sender_lock.calc_script_hash().as_slice()[0..20]);

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

    fn build_acp_outputs(
        &self,
        udt_hash: &Option<H256>,
        to_addr: &Address,
        from_addr: String,
        amount: u128,
        amounts: &mut DetailedAmount,
        capacity_sum: &mut u64,
    ) -> Result<Vec<CellWithData>> {
        let mut ret = self.build_outputs(
            udt_hash,
            to_addr,
            0u64,
            amount,
            &ScriptType::Secp256k1,
            amounts,
            from_addr,
            capacity_sum,
        )?;

        let ckb_needed: u64 = ret[0].cell.capacity().unpack();
        let mut capacity_needed = BigUint::from(ckb_needed);
        let sp_cells = self.get_sp_cells_by_addr(to_addr)?.inner();
        let acp_cells = self.take_sp_cells(&sp_cells, special_cells::ACP);
        let (mut acp_used, mut acp_outputs, mut acp_capacity_sum) = (vec![], vec![], 0);

        self.pool_acp(
            acp_cells,
            &mut capacity_needed,
            &mut Zero::zero(),
            &mut acp_used,
            &mut acp_outputs,
            &mut acp_capacity_sum,
        )?;

        if capacity_needed > Zero::zero() {
            return Err(MercuryError::LackACPCells(to_addr.to_string()).into());
        }

        ret.append(&mut acp_outputs);
        *capacity_sum += acp_capacity_sum;
        ACP_USED_CACHE.insert(thread::current().id(), (acp_used, acp_capacity_sum));

        Ok(ret)
    }

    // Todo: have question here.
    fn build_change_cell(
        &self,
        addr: String,
        udt_hash: Option<H256>,
        ckb_change: u64,
        udt_change: u128,
    ) -> Result<(packed::CellOutput, packed::Bytes)> {
        let address = parse_address(&addr)?;
        let (type_script, data) =
            self.build_type_script(udt_hash, udt_change, &mut Default::default())?;
        let lock_script =
            self.build_lock_script(&address, &ScriptType::Secp256k1, Default::default())?;

        Ok((
            packed::CellOutputBuilder::default()
                .lock(lock_script)
                .type_(type_script.pack())
                .capacity(ckb_change.pack())
                .build(),
            data.pack(),
        ))
    }

    // Todo: can remove this.
    fn add_detailed_amount(
        &self,
        amounts: &mut DetailedAmount,
        to_addr: String,
        capacity: u64,
        script_type: &ScriptType,
    ) {
        match script_type {
            ScriptType::Secp256k1 => amounts.add_ckb_by_owned(capacity),
            ScriptType::AnyoneCanPay => amounts.add_ckb_by_acp(to_addr, capacity),
            ScriptType::Cheque => amounts.add_ckb_lend(capacity),
            _ => unreachable!(),
        };
    }

    fn pool_acp(
        &self,
        acp_cells: Vec<DetailedCell>,
        ckb_needed: &mut BigUint,
        sudt_needed: &mut BigUint,
        acp_used: &mut Vec<packed::OutPoint>,
        acp_outputs: &mut Vec<CellWithData>,
        capacity_sum: &mut u64,
    ) -> Result<()> {
        for detail in acp_cells {
            if ckb_needed.is_zero() && sudt_needed.is_zero() {
                break;
            }

            let (consumable, base) = capacity_detail(&detail)?;
            let acp_data = detail.cell_data.raw_data().to_vec();
            let sudt_amount = decode_udt_amount(&acp_data);

            let capacity = u64_sub(consumable, ckb_needed.clone()) + base;
            let cell = packed::CellOutputBuilder::default()
                .type_(detail.cell_output.type_())
                .lock(detail.cell_output.lock())
                .capacity(capacity.pack())
                .build();

            let mut cell_data = encode_udt_amount(u128_sub(sudt_amount, sudt_needed.clone()));
            cell_data.extend_from_slice(&acp_data[16..]);

            acp_outputs.push(CellWithData::new(cell, Bytes::from(cell_data)));
            acp_used.push(detail.out_point);
            *capacity_sum += capacity;

            *ckb_needed -= consumable.min(ckb_needed.clone().try_into().unwrap());
            *sudt_needed -= sudt_amount.min(sudt_needed.clone().try_into().unwrap());
        }

        Ok(())
    }

    fn pool_ckb<'a, I: Iterator<Item = (&'a DetailedLiveCell, &'a packed::OutPoint)>>(
        &self,
        ckb_iter: I,
        ckb_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        sigs_entry: &mut Vec<SignatureEntry>,
        capacity_sum: &mut u64,
    ) {
        let mut sig_entry = Vec::new();

        println!("{:?}", ckb_needed);

        for (ckb_cell, out_point) in ckb_iter {
            if ckb_needed.is_zero() {
                break;
            }

            println!("{:?}", ckb_cell.cell_output);

            let capacity: u64 = ckb_cell.cell_output.capacity().unpack();
            let consume_ckb = capacity.min(ckb_needed.clone().try_into().unwrap());
            inputs.push(out_point.clone());

            if sig_entry.is_empty() {
                sig_entry.push(SignatureEntry {
                    index: inputs.len() - 1,
                    type_: WitnessType::WitnessArgsLock,
                    message: Default::default(),
                    pub_key: H160::from_slice(&ckb_cell.cell_output.lock().args().raw_data())
                        .unwrap(),
                });
            }

            *ckb_needed -= consume_ckb;
            *capacity_sum += consume_ckb;
        }

        sigs_entry.append(&mut sig_entry);
    }

    fn pool_udt<'a, I: Iterator<Item = (&'a DetailedLiveCell, &'a packed::OutPoint)>>(
        &self,
        udt_iter: I,
        udt_needed: &mut BigUint,
        inputs: &mut Vec<packed::OutPoint>,
        capacity_sum: &mut u64,
        udt_sum: &mut u128,
    ) {
        for (udt_cell, out_point) in udt_iter {
            if udt_needed.is_zero() {
                break;
            }

            let capacity: u64 = udt_cell.cell_output.capacity().unpack();
            let amount = decode_udt_amount(&udt_cell.cell_data.raw_data().to_vec());
            let udt_used = amount.min(udt_needed.clone().try_into().unwrap());
            inputs.push(out_point.clone());

            *udt_needed -= udt_used;
            *udt_sum += udt_used;
            *capacity_sum += capacity;
        }
    }

    fn build_cell_deps(&self, scripts_set: HashSet<ScriptType>) -> Vec<packed::CellDep> {
        scripts_set
            .into_iter()
            .map(|s| self.config.get(s.as_str()).cloned().unwrap().cell_dep)
            .collect()
    }

    fn take_cheque_claimable_cell(
        &self,
        addr: &Address,
        udt_hash: packed::Byte32,
    ) -> Result<Vec<DetailedCell>> {
        let cells = self.get_sp_cells_by_addr(&addr)?.inner();
        let cheque_cells = self.take_sp_cells(&cells, ScriptType::Cheque.as_str());
        let lock_script: packed::Script = addr.payload().into();
        let lock_hash: [u8; 32] = lock_script.calc_script_hash().unpack();

        let ret = cheque_cells
            .iter()
            .filter_map(|cell| {
                if cell.cell_output.lock().args().raw_data()[0..20] == lock_hash.to_vec()
                    && cell
                        .cell_output
                        .type_()
                        .to_opt()
                        .unwrap()
                        .calc_script_hash()
                        == udt_hash
                {
                    Some(cell.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(ret)
    }

    fn take_cheque_redeemable_cell(&self, addr: &Address) -> Result<Vec<DetailedCell>> {
        let cells = self.get_sp_cells_by_addr(&addr)?.inner();
        let cheque_cells = self.take_sp_cells(&cells, ScriptType::Cheque.as_str());
        let lock_script: packed::Script = addr.payload().into();
        let lock_hash: [u8; 32] = lock_script.calc_script_hash().unpack();
        let mut ret = Vec::new();
        let current_epoch = { CURRENT_EPOCH.read().clone().into_u256() };

        for cell in cheque_cells.iter() {
            if cell.cell_output.lock().args().raw_data()[0..20] == lock_hash.to_vec() {
                if current_epoch.clone() - cell.epoch_number.clone() > self.cheque_since {
                    ret.push(cell.clone());
                }
            }
        }

        Ok(ret)
    }

    fn take_sp_cells(&self, cell_list: &[DetailedCell], cell_name: &str) -> Vec<DetailedCell> {
        let script_code_hash = self.config.get(cell_name).unwrap().script.code_hash();
        cell_list
            .iter()
            .filter(|cell| cell.cell_output.lock().code_hash() == script_code_hash)
            .cloned()
            .collect()
    }

    fn _build_cheque_cliam_outputs(
        &self,
        cheque_cell: &DetailedCell,
        sudt_cell: &DetailedLiveCell,
    ) -> Vec<CellWithData> {
        let output_1 = sudt_cell.cell_output.clone();
        let origin_data = sudt_cell.cell_data.clone();
        let origin_amount = decode_udt_amount(&origin_data.raw_data());
        let claimed_amount = decode_udt_amount(&cheque_cell.cell_data.raw_data());
        let mut output_1_data = encode_udt_amount(origin_amount + claimed_amount);
        output_1_data.extend_from_slice(&origin_data.raw_data()[16..]);

        let output_2 = packed::CellOutputBuilder::default().build();

        vec![
            CellWithData::new(output_1, output_1_data.into()),
            CellWithData::new(output_2, Default::default()),
        ]
    }
}
