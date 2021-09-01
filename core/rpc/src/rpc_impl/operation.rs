use crate::block_on;
use crate::rpc_impl::{
    address_to_script, minstant_elapsed, parse_key_address, parse_normal_address,
    CURRENT_BLOCK_NUMBER, USE_HEX_FORMAT,
};
use crate::types::{
    Action, FromAddresses, GenericBlock, GenericTransaction, GetGenericTransactionResponse,
    InnerAccount, InnerAmount, InnerTransferItem, Operation, Source, Status, ToAddress,
    TransferItem,
};
use crate::{error::RpcError, CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, parse_address, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160};
use common::{Address, AddressPayload, MercuryError};
use core_extensions::{
    ckb_balance, script_hash, special_cells, udt_balance, CURRENT_EPOCH, SCRIPT_HASH_EXT_PREFIX,
};
use core_storage::{add_prefix, Batch, Store};

use ckb_jsonrpc_types::{
    AsEpochNumberWithFraction, Status as TransactionStatus, TransactionWithStatus,
};
use ckb_types::core::EpochNumberWithFraction;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

use std::collections::HashMap;
use std::str::FromStr;

impl<S, C> MercuryRpcImpl<S, C>
where
    S: Store,
    C: CkbRpc + Clone + Send + Sync + 'static,
{
    pub(crate) fn inner_register_addresses(
        &self,
        normal_addresses: Vec<String>,
    ) -> Result<Vec<H160>> {
        let mut ret = Vec::new();
        let mut batch = self.store.batch()?;

        for addr in normal_addresses.iter() {
            let script = address_to_script(
                Address::from_str(addr)
                    .map_err(|_| MercuryError::rpc(RpcError::InvalidAddress(addr.to_string())))?
                    .payload(),
            );
            let script_hash = blake2b_160(script.as_slice());
            let key = add_prefix(
                *SCRIPT_HASH_EXT_PREFIX,
                script_hash::Key::ScriptHash(script_hash).into_vec(),
            );

            batch.put_kv(key, script_hash::Value::Script(&script))?;
            ret.push(H160(script_hash));
        }

        batch.commit()?;

        Ok(ret)
    }

    pub(crate) fn inner_get_generic_block(
        &self,
        txs: Vec<packed::Transaction>,
        block_num: BlockNumber,
        block_hash: H256,
        parent_hash: H256,
        timestamp: u64,
        current_num: BlockNumber,
    ) -> Result<GenericBlock> {
        let mut res: Vec<GenericTransaction> = Vec::new();

        for tx in txs.into_iter() {
            let tx_hash = tx.calc_tx_hash();
            res.push(
                self.inner_get_generic_transaction(
                    tx,
                    tx_hash.unpack(),
                    TransactionStatus::Committed,
                    Some(block_hash.clone()),
                    Some(block_num),
                    Some(current_num),
                )?
                .into(),
            );
        }

        Ok(GenericBlock::new(
            block_num,
            block_hash,
            parent_hash,
            timestamp,
            res,
        ))
    }

    pub(crate) fn inner_get_generic_transaction(
        &self,
        tx: packed::Transaction,
        tx_hash: H256,
        tx_status: TransactionStatus,
        block_hash: Option<H256>,
        block_num: Option<BlockNumber>,
        confirmed_number: Option<BlockNumber>,
    ) -> Result<GetGenericTransactionResponse> {
        let mut id = 0;
        let mut ops = Vec::new();
        let (mut out_point_map, mut tx_hashes) = (HashMap::new(), vec![]);
        let tx_view = tx.into_view();
        let now = minstant::now();

        for input in tx_view.inputs().into_iter() {
            // The input cell of cellbase is zero tx hash, skip it.
            if input.previous_output().tx_hash().is_zero() {
                continue;
            }

            if let Some(cell) = self.get_detailed_live_cell(&input.previous_output())? {
                let mut op = self.build_operation(
                    &mut id,
                    block_num.unwrap(),
                    &cell.cell_output,
                    &cell.cell_data,
                    &input.previous_output(),
                    true,
                )?;
                ops.append(&mut op);
                id += 1;
            } else {
                let out_point = input.previous_output();
                let hash: H256 = out_point.tx_hash().unpack();
                let index: u32 = out_point.index().unpack();
                tx_hashes.push(hash);
                out_point_map.insert(out_point.tx_hash(), index as usize);
            }
        }

        if !tx_hashes.is_empty() {
            for tx in block_on!(self, get_transactions, tx_hashes)?.into_iter() {
                let tx: packed::Transaction = tx.unwrap().transaction.inner.into();
                let tx_view = tx.into_view();
                let index = *out_point_map.get(&tx_view.hash()).unwrap();
                let output = tx_view.output(index).unwrap();
                let data = tx_view.outputs_data().get_unchecked(index);

                let mut op = self.build_operation(
                    &mut id,
                    block_num.unwrap(),
                    &output,
                    &data,
                    &packed::OutPointBuilder::default()
                        .tx_hash(tx_view.hash())
                        .index((index as u32).pack())
                        .build(),
                    true,
                )?;
                ops.append(&mut op);
                id += 1;
            }
        }

        let tx_view_hash = tx_view.hash();
        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let data = data.pack();
            let mut op = self.build_operation(
                &mut id,
                block_num.unwrap(),
                &cell,
                &data,
                &packed::OutPointBuilder::default()
                    .tx_hash(tx_view_hash.clone())
                    .index((idx as u32).pack())
                    .build(),
                false,
            )?;
            ops.append(&mut op);
            id += 1;
        }

        let generic_tx = GenericTransaction::new(tx_hash, ops);

        log::debug!("inner build cost {}", minstant_elapsed(now));

        Ok(GetGenericTransactionResponse::new(
            generic_tx,
            tx_status,
            block_hash,
            block_num,
            confirmed_number,
        ))
    }

    #[allow(clippy::if_same_then_else, clippy::collapsible_else_if)]
    pub(crate) fn build_operation(
        &self,
        id: &mut u32,
        block_number: BlockNumber,
        cell: &packed::CellOutput,
        cell_data: &packed::Bytes,
        cell_out_point: &packed::OutPoint,
        is_input: bool,
    ) -> Result<Vec<Operation>> {
        let mut ret = Vec::new();
        let normal_address = Address::new(
            self.net_ty,
            AddressPayload::from_script(&cell.lock(), self.net_ty),
        );

        if self.is_sudt(&cell.type_()) {
            let mut udt_amount = InnerAmount {
                value: self.get_udt_amount(is_input, cell_data.raw_data()),
                udt_hash: cell.type_().to_opt().map(|s| s.calc_script_hash().unpack()),
                status: Status::Fixed(block_number),
            };

            let ckb_amount = InnerAmount {
                value: self.get_ckb_amount(is_input, cell),
                udt_hash: None,
                status: Status::Fixed(block_number),
            };

            if self.is_secp256k1(&cell.lock()) {
                let key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&cell.lock().args().raw_data()[0..20]).unwrap(),
                );
                ret.push(Operation::new(
                    *id,
                    key_addr.to_string(),
                    normal_address.to_string(),
                    udt_amount.into(),
                ));

                *id += 1;
                ret.push(Operation::new(
                    *id,
                    key_addr.to_string(),
                    normal_address.to_string(),
                    ckb_amount.into(),
                ));
            } else if self.is_acp(&cell.lock()) {
                let key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&cell.lock().args().raw_data()[0..20]).unwrap(),
                );

                ret.push(Operation::new(
                    *id,
                    key_addr.to_string(),
                    normal_address.to_string(),
                    udt_amount.into(),
                ));

                *id += 1;
                ret.push(Operation::new(
                    *id,
                    key_addr.to_string(),
                    normal_address.to_string(),
                    ckb_amount.into(),
                ));
            } else if self.is_cheque(&cell.lock()) {
                let epoch_number = block_on!(
                    self,
                    get_block_by_number,
                    block_number,
                    **USE_HEX_FORMAT.load()
                )?
                .unwrap()
                .header
                .inner
                .epoch;
                let rational_number = EpochNumberWithFraction::new(
                    epoch_number.epoch_number(),
                    epoch_number.epoch_index(),
                    epoch_number.epoch_length(),
                )
                .to_rational();

                let sender_key_addr = if let Ok(sender_lock) =
                    self.get_script_by_hash(to_fixed_array(&cell.lock().args().raw_data()[20..40]))
                {
                    self.pubkey_to_key_address(
                        H160::from_slice(&sender_lock.args().raw_data()[0..20]).unwrap(),
                    )
                    .to_string()
                } else {
                    H160::from_slice(&cell.lock().args().raw_data()[20..40])
                        .unwrap()
                        .to_string()
                };

                let receiver_key_addr = if let Ok(sender_lock) =
                    self.get_script_by_hash(to_fixed_array(&cell.lock().args().raw_data()[0..20]))
                {
                    self.pubkey_to_key_address(
                        H160::from_slice(&sender_lock.args().raw_data()[0..20]).unwrap(),
                    )
                    .to_string()
                } else {
                    H160::from_slice(&cell.lock().args().raw_data()[0..20])
                        .unwrap()
                        .to_string()
                };

                let is_spent = if is_input {
                    true
                } else {
                    let sp_cells = self.get_sp_cells_by_addr(&parse_address(&sender_key_addr)?)?;
                    !sp_cells
                        .0
                        .iter()
                        .any(|cell| &cell.out_point == cell_out_point)
                };

                let (key_address, status) = if is_input {
                    let tx_hash: H256 = cell_out_point.tx_hash().unpack();
                    let block_hash = block_on!(self, get_transactions, vec![tx_hash])?
                        .get(0)
                        .cloned()
                        .unwrap()
                        .unwrap()
                        .tx_status
                        .block_hash
                        .unwrap();
                    let block =
                        block_on!(self, get_block, block_hash, **USE_HEX_FORMAT.load())?.unwrap();
                    let epoch = block.header.inner.epoch;

                    let addr = if rational_number
                        - EpochNumberWithFraction::new(
                            epoch.epoch_number(),
                            epoch.epoch_index(),
                            epoch.epoch_length(),
                        )
                        .to_rational()
                        > self.cheque_since
                    {
                        sender_key_addr
                    } else {
                        receiver_key_addr
                    };

                    (addr, Status::Fixed(block_number))
                } else {
                    if is_spent {
                        let search_key = build_search_args(block_number, cell);

                        let tx_hashes = self
                            .get_transactions(search_key, None)?
                            .objects
                            .iter()
                            .map(|obj| obj.tx_hash.clone())
                            .collect::<Vec<_>>();
                        let txs = block_on!(self, get_transactions, tx_hashes)?
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();
                        let consumed_tx_hash = find_input_from_txs(txs, cell_out_point);
                        let consumed_block =
                            block_on!(self, get_block, consumed_tx_hash, **USE_HEX_FORMAT.load())?
                                .unwrap();
                        let epoch = consumed_block.header.inner.epoch;

                        let addr = if EpochNumberWithFraction::new(
                            epoch.epoch_number(),
                            epoch.epoch_index(),
                            epoch.epoch_length(),
                        )
                        .to_rational()
                            - rational_number
                            > self.cheque_since
                        {
                            sender_key_addr
                        } else {
                            receiver_key_addr
                        };

                        (
                            addr,
                            Status::Fixed(consumed_block.header.inner.number.into()),
                        )
                    } else {
                        if CURRENT_EPOCH.read().clone() - rational_number > self.cheque_since {
                            (sender_key_addr, Status::Fixed(block_number))
                        } else {
                            (receiver_key_addr, Status::Claimable(block_number))
                        }
                    }
                };

                ret.push(Operation::new(
                    *id,
                    key_address,
                    normal_address.to_string(),
                    ckb_amount.into(),
                ));
                udt_amount.status = status;
            } else {
                let addr = self.generate_long_address(cell.lock());
                udt_amount.status = Status::Fixed(block_number);

                ret.push(Operation::new(
                    *id,
                    addr.to_string(),
                    normal_address.to_string(),
                    udt_amount.into(),
                ));

                *id += 1;
                ret.push(Operation::new(
                    *id,
                    addr.to_string(),
                    normal_address.to_string(),
                    ckb_amount.into(),
                ));
            }
        } else {
            let mut amount = InnerAmount {
                value: self.get_ckb_amount(is_input, cell),
                udt_hash: None,
                status: Status::Fixed(block_number),
            };

            if self.is_secp256k1(&cell.lock()) {
                let key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&cell.lock().args().raw_data()[0..20]).unwrap(),
                );

                if cell_data.is_empty() {
                    amount.status = Status::Fixed(block_number);
                    ret.push(Operation::new(
                        *id,
                        key_addr.to_string(),
                        normal_address.to_string(),
                        amount.into(),
                    ));
                } else {
                    ret.push(Operation::new(
                        *id,
                        key_addr.to_string(),
                        normal_address.to_string(),
                        amount.into(),
                    ));
                }
            } else if self.is_acp(&cell.lock()) {
                let key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&cell.lock().args().raw_data()[0..20]).unwrap(),
                );

                ret.push(Operation::new(
                    *id,
                    key_addr.to_string(),
                    normal_address.to_string(),
                    amount.into(),
                ));
            } else if self.is_cheque(&cell.lock()) {
                let sender_lock = self
                    .get_script_by_hash(to_fixed_array(&cell.lock().args().raw_data()[20..40]))?;
                let sender_key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&sender_lock.args().raw_data()[0..20]).unwrap(),
                );

                ret.push(Operation::new(
                    *id,
                    sender_key_addr.to_string(),
                    normal_address.to_string(),
                    amount.into(),
                ));
            } else {
                let addr = self.generate_long_address(cell.lock());
                ret.push(Operation::new(
                    *id,
                    addr.to_string(),
                    addr.to_string(),
                    amount.into(),
                ));
            }
        }

        Ok(ret)
    }

    fn get_ckb_amount(&self, is_input: bool, cell: &packed::CellOutput) -> BigInt {
        let capacity: u64 = cell.capacity().unpack();
        if is_input {
            BigInt::from(capacity) * (-1)
        } else {
            BigInt::from(capacity)
        }
    }

    fn get_udt_amount(&self, is_input: bool, data: Bytes) -> BigInt {
        let amount = BigInt::from(decode_udt_amount(&data));
        if is_input {
            amount * (-1)
        } else {
            amount
        }
    }

    pub(crate) fn get_tx_block_num_and_hash(
        &self,
        tx_hash: [u8; 32],
        tx_status: TransactionStatus,
    ) -> Result<(Option<BlockNumber>, Option<H256>)> {
        match tx_status {
            TransactionStatus::Committed => {
                let key = script_hash::Key::TxHash(tx_hash).into_vec();
                let bytes = self.store_get(*SCRIPT_HASH_EXT_PREFIX, key)?.unwrap();
                let block_num = BlockNumber::from_be_bytes(to_fixed_array(&bytes[0..8]));
                let block_hash = H256::from_slice(&bytes[8..40]).unwrap();
                Ok((Some(block_num), Some(block_hash)))
            }
            _ => Ok((None, None)),
        }
    }

    pub(crate) fn is_secp256k1(&self, script: &packed::Script) -> bool {
        let config = self.config.get(ckb_balance::SECP256K1_BLAKE160).unwrap();
        config.script.hash_type() == script.hash_type()
            && config.script.code_hash() == script.code_hash()
    }

    pub(crate) fn is_acp(&self, script: &packed::Script) -> bool {
        let config = self.config.get(special_cells::ACP).unwrap();
        config.script.hash_type() == script.hash_type()
            && config.script.code_hash() == script.code_hash()
    }

    pub(crate) fn is_cheque(&self, script: &packed::Script) -> bool {
        let config = self.config.get(special_cells::CHEQUE).unwrap();
        config.script.hash_type() == script.hash_type()
            && config.script.code_hash() == script.code_hash()
    }

    pub(crate) fn is_sudt(&self, script: &packed::ScriptOpt) -> bool {
        if script.is_none() {
            return false;
        }

        let script = script.to_opt().unwrap();
        let config = self.config.get(udt_balance::SUDT).unwrap();
        config.script.hash_type() == script.hash_type()
            && config.script.code_hash() == script.code_hash()
    }

    pub(crate) fn pubkey_to_key_address(&self, pubkey_hash: H160) -> Address {
        Address::new(
            self.net_ty,
            AddressPayload::from_pubkey_hash(self.net_ty, pubkey_hash),
        )
    }

    pub(crate) fn generate_long_address(&self, script: packed::Script) -> Address {
        Address::new(self.net_ty, AddressPayload::from(script))
    }

    pub(crate) fn handle_from_addresses(&self, addresses: FromAddresses) -> Result<InnerAccount> {
        match addresses {
            FromAddresses::KeyAddresses(addrs) => {
                let mut idents = Vec::new();
                for a in addrs.key_addresses.iter() {
                    let _ = parse_key_address(a)?;
                    idents.push(a.clone());
                }

                Ok(InnerAccount {
                    idents,
                    scripts: addrs.source.to_scripts(),
                })
            }

            FromAddresses::NormalAddresses(addrs) => {
                let mut idents = Vec::new();
                let mut prev_source = Source::Unconstrained;

                for (idx, a) in addrs.iter().enumerate() {
                    let addr = parse_normal_address(a)?;
                    let script: packed::Script = addr.payload().into();
                    let (key_addr, source) = if self.is_acp(&script) || self.is_secp256k1(&script) {
                        let key_addr = self.pubkey_to_key_address(
                            H160::from_slice(&script.args().raw_data()[0..20]).unwrap(),
                        );

                        (key_addr.to_string(), Source::Unconstrained)
                    } else if self.is_cheque(&script) {
                        let key_addr = Address::new(
                            self.net_ty,
                            self.get_script_by_hash(to_fixed_array(
                                &script.args().raw_data()[0..20],
                            ))?
                            .into(),
                        );

                        (key_addr.to_string(), Source::Fleeting)
                    } else {
                        return Err(MercuryError::rpc(RpcError::InvalidNormalAddress(
                            addr.to_string(),
                        ))
                        .into());
                    };

                    if idx == 0 {
                        prev_source = source;
                    }

                    if source != prev_source {
                        return Err(MercuryError::rpc(RpcError::FromNormalAddressIsMixed).into());
                    }

                    idents.push(key_addr)
                }

                Ok(InnerAccount {
                    idents,
                    scripts: prev_source.to_scripts(),
                })
            }
        }
    }

    pub(crate) fn handle_to_items(
        &self,
        items: Vec<TransferItem>,
        is_udt: bool,
    ) -> Result<Vec<InnerTransferItem>> {
        let mut ret = Vec::new();
        for item in items.iter() {
            let account = match &item.to {
                ToAddress::KeyAddress(addr) => {
                    let _ = parse_key_address(&addr.key_address)?;
                    InnerAccount {
                        idents: vec![addr.key_address.clone()],
                        scripts: addr.action.to_scripts(is_udt),
                    }
                }

                ToAddress::NormalAddress(addr) => {
                    let origin_addr = Address::from_str(&addr)
                        .map_err(|_| MercuryError::rpc(RpcError::InvalidAddress(addr.clone())))?;
                    let script = address_to_script(origin_addr.payload());
                    if self.is_secp256k1(&script) {
                        InnerAccount {
                            idents: vec![self
                                .pubkey_to_key_address(
                                    H160::from_slice(&origin_addr.payload().args()[0..20]).unwrap(),
                                )
                                .to_string()],
                            scripts: Action::PayByFrom.to_scripts(is_udt),
                        }
                    } else if self.is_acp(&script) {
                        InnerAccount {
                            idents: vec![self
                                .pubkey_to_key_address(
                                    H160::from_slice(&script.args().raw_data()[0..20]).unwrap(),
                                )
                                .to_string()],
                            scripts: Action::PayByTo.to_scripts(is_udt),
                        }
                    } else {
                        return Err(MercuryError::rpc(RpcError::InvalidAddress(
                            origin_addr.to_string(),
                        ))
                        .into());
                    }
                }
            };

            ret.push(InnerTransferItem {
                to: account,
                amount: item.amount,
            });
        }

        Ok(ret)
    }
}

fn build_search_args(
    block_number: u64,
    cell: &packed::CellOutput,
) -> ckb_indexer::service::SearchKey {
    let block_range_from: ckb_jsonrpc_types::Uint64 = block_number.into();
    let block_range_to: ckb_jsonrpc_types::Uint64 = (**CURRENT_BLOCK_NUMBER.load()).into();
    let script = ckb_jsonrpc_types::Script::from(cell.lock());
    let script_type = ckb_indexer::service::ScriptType::Lock;
    let mut range = [ckb_jsonrpc_types::Uint64::from(0); 2];
    range.copy_from_slice(&[block_range_from, block_range_to]);
    let filter = ckb_indexer::service::SearchKeyFilter {
        block_range: Some(range),
        ..Default::default()
    };

    ckb_indexer::service::SearchKey {
        script,
        script_type,
        filter: Some(filter),
    }
}

fn find_input_from_txs(txs: Vec<TransactionWithStatus>, out_point: &packed::OutPoint) -> H256 {
    for tx in txs.iter() {
        for input in tx.transaction.inner.inputs.iter() {
            let inner_op: packed::OutPoint = input.previous_output.clone().into();
            if &inner_op == out_point {
                return tx.tx_status.block_hash.clone().unwrap();
            }
        }
    }
    H256::default()
}
