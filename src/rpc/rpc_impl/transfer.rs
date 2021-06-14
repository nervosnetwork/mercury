use crate::error::MercuryError;
use crate::extensions::{special_cells, udt_balance, DetailedCell, CURRENT_EPOCH, UDT_EXT_PREFIX};
use crate::rpc::rpc_impl::{
    address_to_script, ckb_iter, udt_iter, MercuryRpcImpl, ACP_USED_CACHE, BYTE_SHANNONS,
    CHEQUE_CELL_CAPACITY, MIN_CKB_CAPACITY, STANDARD_SUDT_CAPACITY, TX_POOL_CACHE,
};
use crate::rpc::types::{
    details_split_off, CellWithData, DetailedAmount, InnerAccount, InnerTransferItem, InputConsume,
    ScriptType, SignatureEntry, TransactionCompletionResponse, WalletInfo, CHEQUE, SECP256K1,
};
use crate::utils::{
    decode_udt_amount, encode_udt_amount, parse_address, u128_sub, unwrap_only_one,
};

use anyhow::Result;
use ckb_indexer::{indexer::DetailedLiveCell, store::Store};
use ckb_sdk::Address;
use ckb_types::core::{ScriptHashType, TransactionBuilder, TransactionView};
use ckb_types::{bytes::Bytes, constants::TX_VERSION, packed, prelude::*, H256};
use num_bigint::BigUint;
use num_traits::identities::Zero;

use std::collections::{HashMap, HashSet};
use std::{convert::TryInto, iter::Iterator, thread};

impl<S: Store> MercuryRpcImpl<S> {
    pub(crate) fn inner_transfer_complete(
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
        address: String,
        udt_info: Vec<WalletInfo>,
        fee: u64,
    ) -> Result<TransactionCompletionResponse> {
        let mut capacity_needed = fee + MIN_CKB_CAPACITY;
        let (mut inputs, mut outputs, mut sigs_entry) = (vec![], vec![], HashMap::new());
        let addr_payload = parse_address(&address)?.payload().to_owned();
        let pubkey_hash = addr_payload.args();
        let lock_script = address_to_script(&addr_payload);
        let acp_lock = self
            .config
            .get(special_cells::ACP)
            .ok_or_else(|| MercuryError::MissingConfig(special_cells::ACP.to_string()))?
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
            return Err(MercuryError::CkbIsNotEnough(address).into());
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
        let (mut sigs_entry, mut acp_sigs_entry) = (HashMap::new(), vec![]);

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
                return Err(MercuryError::CkbIsNotEnough(from.idents[0].clone()).into());
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
                return Err(MercuryError::UDTIsNotEnough(from.idents[0].clone()).into());
            }

            if ckb_needed > Zero::zero() {
                return Err(MercuryError::CkbIsNotEnough(from.idents[0].clone()).into());
            }

            if let Some((_id, mut acp_cells)) = (*ACP_USED_CACHE).remove(&thread::current().id()) {
                inputs.append(&mut acp_cells);
            }
        }

        let mut sigs_entry = sigs_entry.into_iter().map(|(_k, v)| v).collect::<Vec<_>>();
        sigs_entry.append(&mut acp_sigs_entry);
        sigs_entry.sort();

        Ok((InputConsume::new(capacity_sum, udt_sum), sigs_entry))
    }

    fn _build_cheque_claim(
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
                let cells = self._take_cheque_claimable_cell(&addr, hash.pack())?;
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
                let cells = self.get_cells_by_lock_script(&script)?;
                let sudt_cell = udt_iter(&cells, hash.pack()).next();

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
                let cells = self._take_cheque_redeemable_cell(&addr)?;
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
    ) -> Result<CellWithData> {
        if script.is_acp() {
            return self.build_acp_outputs(udt_hash, to_addr, udt_amount, amounts);
        }

        if script.is_my_acp() {
            return self.build_my_acp_outputs(udt_hash, to_addr, udt_amount, amounts);
        }

        let (type_script, data) = self.build_type_script(udt_hash.clone(), udt_amount)?;
        let lock_script = self.build_lock_script(to_addr, script, from_addr)?;
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
                .ok_or_else(|| MercuryError::UDTInexistence(hex::encode(hash.as_bytes())))?;
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
        to_addr: &Address,
        script: &ScriptType,
        from_addr: String,
    ) -> Result<packed::Script> {
        let script_builder = packed::ScriptBuilder::default();

        let script = match script {
            ScriptType::Secp256k1 => address_to_script(to_addr.payload()),
            ScriptType::Cheque => {
                let code_hash = self
                    .config
                    .get(CHEQUE)
                    .ok_or_else(|| MercuryError::MissingConfig(CHEQUE.to_string()))?
                    .script
                    .code_hash();
                let receiver_lock = to_addr.payload().args();
                let sender_lock = parse_address(&from_addr)?.payload().args();
                let mut lock_args = Vec::from(&receiver_lock.pack().as_slice()[4..24]);
                lock_args.extend_from_slice(&sender_lock.pack().as_slice()[4..24]);

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
            .ok_or_else(|| MercuryError::MissingConfig(special_cells::ACP.to_string()))?
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
                MercuryError::MissingACPCell(to_addr.to_string(), hex::encode(sudt_hash.as_ref()))
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
        let (type_script, data) = self.build_type_script(udt_hash.clone(), udt_change)?;
        let lock_script =
            self.build_lock_script(&address, &ScriptType::Secp256k1, Default::default())?;
        let ckb_capacity = if udt_hash.is_some() {
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
                sigs_entry.insert(addr.clone(), SignatureEntry::new(inputs.len() - 1, addr));
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
                sigs_entry.insert(addr.clone(), SignatureEntry::new(inputs.len() - 1, addr));
            }
        }
    }

    fn build_cell_deps(&self, scripts_set: HashSet<String>) -> Vec<packed::CellDep> {
        scripts_set
            .into_iter()
            .map(|s| self.config.get(s.as_str()).cloned().unwrap().cell_dep)
            .collect()
    }

    fn _take_cheque_claimable_cell(
        &self,
        addr: &Address,
        udt_hash: packed::Byte32,
    ) -> Result<Vec<DetailedCell>> {
        let cells = self.get_sp_cells_by_addr(&addr)?.inner();
        let cheque_cells = self.take_sp_cells(&cells, ScriptType::Cheque.as_str())?;
        let lock_script = address_to_script(addr.payload());
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

    fn _take_cheque_redeemable_cell(&self, addr: &Address) -> Result<Vec<DetailedCell>> {
        let cells = self.get_sp_cells_by_addr(&addr)?.inner();
        let cheque_cells = self.take_sp_cells(&cells, ScriptType::Cheque.as_str())?;
        let lock_script: packed::Script = addr.payload().into();
        let lock_hash: [u8; 32] = lock_script.calc_script_hash().unpack();
        let mut ret = Vec::new();
        let current_epoch = { CURRENT_EPOCH.read().clone().into_u256() };

        for cell in cheque_cells.iter() {
            if cell.cell_output.lock().args().raw_data()[0..20] == lock_hash.to_vec()
                && current_epoch.clone() - cell.epoch_number.clone() > self._cheque_since
            {
                ret.push(cell.clone());
            }
        }

        Ok(ret)
    }

    fn take_sp_cells(
        &self,
        cell_list: &[DetailedCell],
        cell_name: &str,
    ) -> Result<Vec<DetailedCell>> {
        let script_code_hash = self
            .config
            .get(cell_name)
            .ok_or_else(|| MercuryError::MissingConfig(cell_name.to_string()))?
            .script
            .code_hash();

        Ok(cell_list
            .iter()
            .filter(|cell| cell.cell_output.lock().code_hash() == script_code_hash)
            .cloned()
            .collect())
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
}

fn read_tx_pool_cache() -> HashSet<packed::OutPoint> {
    let cache = TX_POOL_CACHE.read();
    cache.clone()
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
