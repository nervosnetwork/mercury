use crate::types::{
    GenericBlock, GenericTransaction, GetGenericTransactionResponse, InnerAmount, Operation, Status,
};
use crate::{error::RpcError, rpc_impl::address_to_script, CkbRpc, MercuryRpcImpl};

use common::utils::{decode_udt_amount, to_fixed_array};
use common::{anyhow::Result, hash::blake2b_160};
use common::{Address, AddressPayload, MercuryError};
use core_extensions::{
    ckb_balance, script_hash, special_cells, udt_balance, SCRIPT_HASH_EXT_PREFIX,
};
use core_storage::{add_prefix, Batch, Store};

use ckb_jsonrpc_types::Status as TransactionStatus;
use ckb_types::{bytes::Bytes, core::BlockNumber, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

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
                    .map_err(|_| {
                        MercuryError::rpc(RpcError::InvalidRegisterAddress(addr.to_string()))
                    })?
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
        let tx_view = tx.into_view();

        for input in tx_view.inputs().into_iter() {
            // The input cell of cellbase is zero tx hash, skip it.
            if input.previous_output().tx_hash().is_zero() {
                continue;
            }

            let cell = self
                .get_detailed_live_cell(&input.previous_output())?
                .unwrap();
            let mut op = self.build_operation(&mut id, &cell.cell_output, &cell.cell_data, true)?;
            ops.append(&mut op);
            id += 1;
        }

        for (cell, data) in tx_view.outputs_with_data_iter() {
            let data = data.pack();
            let mut op = self.build_operation(&mut id, &cell, &data, false)?;
            ops.append(&mut op);
            id += 1;
        }

        let generic_tx = GenericTransaction::new(tx_hash, ops);

        Ok(GetGenericTransactionResponse::new(
            generic_tx,
            tx_status,
            block_hash,
            block_num,
            confirmed_number,
        ))
    }

    #[allow(clippy::if_same_then_else)]
    pub(crate) fn build_operation(
        &self,
        id: &mut u32,
        cell: &packed::CellOutput,
        cell_data: &packed::Bytes,
        is_input: bool,
    ) -> Result<Vec<Operation>> {
        let mut ret = Vec::new();
        let normal_address = Address::new(self.net_ty, cell.lock().into());

        if self.is_sudt(&cell.type_()) {
            let mut udt_amount = InnerAmount {
                value: self.get_udt_amount(is_input, cell_data.raw_data()),
                udt_hash: cell.type_().to_opt().map(|s| s.calc_script_hash().unpack()),
                status: Status::Unconstrained,
            };
            let ckb_amount = InnerAmount {
                value: self.get_ckb_amount(is_input, cell),
                udt_hash: None,
                status: Status::Locked,
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
                let mut script_hash = [0u8; 20];
                script_hash.copy_from_slice(&cell.lock().args().raw_data()[20..40]);
                let sender_lock = self.get_script_by_hash(script_hash)?;

                script_hash.copy_from_slice(&cell.lock().args().raw_data()[0..20]);
                let receiver_lock = self.get_script_by_hash(script_hash)?;

                let sender_key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&sender_lock.args().raw_data()[0..20]).unwrap(),
                );
                let receiver_key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&receiver_lock.args().raw_data()[0..20]).unwrap(),
                );

                ret.push(Operation::new(
                    *id,
                    sender_key_addr.to_string(),
                    normal_address.to_string(),
                    ckb_amount.into(),
                ));

                *id += 1;
                udt_amount.status = Status::Fleeting;
                ret.push(Operation::new(
                    *id,
                    receiver_key_addr.to_string(),
                    normal_address.to_string(),
                    udt_amount.into(),
                ));
            } else {
                let addr = self.generate_long_address(cell.lock());
                udt_amount.status = Status::Locked;

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
                status: Status::Locked,
            };

            if self.is_secp256k1(&cell.lock()) {
                let key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&cell.lock().args().raw_data()[0..20]).unwrap(),
                );

                if cell_data.is_empty() {
                    amount.status = Status::Unconstrained;
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
                let mut script_hash = [0u8; 20];
                script_hash.copy_from_slice(&cell.lock().args().raw_data()[20..40]);
                let sender_lock = self.get_script_by_hash(script_hash)?;
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
                let block_num = BlockNumber::from_be_bytes(to_fixed_array(&bytes[0..16]));
                let block_hash = H256::from_slice(&bytes[16..48]).unwrap();
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
        Address::new(self.net_ty, AddressPayload::from_pubkey_hash(pubkey_hash))
    }

    pub(crate) fn generate_long_address(&self, script: packed::Script) -> Address {
        Address::new(self.net_ty, AddressPayload::from(script))
    }
}
