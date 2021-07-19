use crate::types::{
    GenericBlock, GenericTransaction, GenericTransactionWithStatus, InnerAmount, Operation, Status,
};
use crate::{CkbRpc, MercuryRpcImpl};

use common::anyhow::Result;
use common::utils::decode_udt_amount;
use common::{Address, AddressPayload};
use core_extensions::{ckb_balance, special_cells, udt_balance};
use core_storage::Store;

use ckb_jsonrpc_types::Status as TransactionStatus;
use ckb_types::{bytes::Bytes, packed, prelude::*, H160, H256};
use num_bigint::BigInt;

impl<S, C> MercuryRpcImpl<S, C>
where
    S: Store,
    C: CkbRpc + Clone + Send + Sync + 'static,
{
    pub(crate) fn inner_get_generic_block(
        &self,
        txs: Vec<packed::Transaction>,
        block_num: u64,
        block_hash: H256,
        parent_hash: H256,
        timestamp: u64,
    ) -> Result<GenericBlock> {
        let mut res: Vec<GenericTransaction> = Vec::new();

        for tx in txs.into_iter() {
            let tx_hash = tx.calc_tx_hash();
            res.push(
                self.inner_get_generic_transaction(
                    tx,
                    tx_hash.unpack(),
                    TransactionStatus::Committed,
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
    ) -> Result<GenericTransactionWithStatus> {
        let mut id = 0;
        let mut ops = Vec::new();
        let tx_view = tx.clone().into_view();

        for input in tx_view.inputs().into_iter() {
            let cell = self
                .get_detailed_live_cell(&input.previous_output())?
                .unwrap();
            let mut op = self.build_operation(
                &mut id,
                &cell.cell_output,
                &input.previous_output(),
                &cell.cell_data,
                true,
                &tx,
            )?;
            ops.append(&mut op);
            id += 1;
        }

        // The out point is useless when the cell is in output.
        for (idx, (cell, data)) in tx_view.outputs_with_data_iter().enumerate() {
            let data = data.pack();
            let mut op = self.build_operation(
                &mut id,
                &cell,
                &packed::OutPointBuilder::default()
                    .tx_hash(tx_hash.pack())
                    .index((idx as u32).pack())
                    .build(),
                &data,
                false,
                &tx,
            )?;
            ops.append(&mut op);
            id += 1;
        }

        let generic_tx = GenericTransaction::new(tx_hash, ops);

        Ok(GenericTransactionWithStatus::new(generic_tx, tx_status))
    }

    #[allow(clippy::if_same_then_else)]
    pub(crate) fn build_operation(
        &self,
        id: &mut u32,
        cell: &packed::CellOutput,
        out_point: &packed::OutPoint,
        cell_data: &packed::Bytes,
        is_input: bool,
        tx: &packed::Transaction,
    ) -> Result<Vec<Operation>> {
        let mut ret = Vec::new();
        let normal_address = Address::new(self.net_ty, cell.lock().into());

        if self.is_sudt(&cell.type_()) {
            let udt_amount = InnerAmount {
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
                let sender_lock =
                    self.get_cheque_sender_lock(out_point, &cell.lock().args().raw_data()[20..40])?;
                let sender_key_addr = self.pubkey_to_key_address(
                    H160::from_slice(&sender_lock.args().raw_data()[0..20]).unwrap(),
                );

                ret.push(Operation::new(
                    *id,
                    sender_key_addr.to_string(),
                    normal_address.to_string(),
                    ckb_amount.into(),
                ));
            } else {
                let addr = self.generate_long_address(cell.lock());
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
                todo!()
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
