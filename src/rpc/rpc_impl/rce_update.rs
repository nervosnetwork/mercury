use crate::error::MercuryError;
use crate::extensions::rce_validator;
use crate::extensions::rce_validator::generated::xudt_rce;
use crate::rpc::rpc_impl::{change_witness, swap_item, MAX_RCE_RULE_NUM};
use crate::rpc::types::{InnerRCRule, RCECellPair, RCState, SMTUpdateItem};
use crate::rpc::MercuryRpcImpl;

use anyhow::Result;
use ckb_indexer::store::Store;
use ckb_jsonrpc_types::{Transaction, TransactionView};
use ckb_types::{bytes::Bytes, packed, prelude::*};

impl<S: Store> MercuryRpcImpl<S> {
    // TODO: can do perf here
    fn is_rce_cell(&self, cell: &packed::CellOutput) -> bool {
        if let Some(rce_config) = self.config.get(rce_validator::RCE) {
            if let Some(type_script) = cell.type_().to_opt() {
                if type_script.code_hash() == rce_config.script.code_hash()
                    && rce_config.script.hash_type() == type_script.hash_type()
                {
                    return true;
                }
            }
        }

        false
    }

    // TODO: can do perf here
    fn extract_rce_cells(&self, transaction: &Transaction) -> Result<Vec<RCECellPair>> {
        let mut ret = Vec::new();

        for (idx, (input, output)) in transaction
            .inputs
            .iter()
            .zip(transaction.outputs.iter())
            .enumerate()
        {
            if let Some(cell) = self.get_detailed_live_cell(&input.previous_output)? {
                if self.is_rce_cell(&cell.cell_output) {
                    let output: packed::CellOutput = output.clone().into();

                    if !self.is_rce_cell(&output) {
                        return Err(MercuryError::InvalidOutputCellWhenUpdateRCE.into());
                    }

                    ret.push(RCECellPair {
                        index: idx,
                        input: cell,
                        output,
                    });
                }
            } else {
                return Err(
                    MercuryError::CannotFindCellByOutPoint(input.previous_output.clone()).into(),
                );
            }
        }

        Ok(ret)
    }

    fn build_proof(&self, smt: &SMT, update_items: &[SMTUpdateItem]) -> Result<Vec<u8>> {
        let mut keys = Vec::new();
        let mut leaves = Vec::new();

        for item in update_items.iter() {
            let key: smt::H256 = item.key.0.into();
            let val = smt
                .get(&key)
                .map_err(|e| MercuryError::SMTError(e.to_string()))?;
            keys.push(key);
            leaves.push((key, val));
        }

        self.build_merkle_proof(smt, keys, leaves)
    }

    fn build_merkle_proof(
        &self,
        smt: &SMT,
        keys: Vec<smt::H256>,
        leaves: Vec<(smt::H256, smt::H256)>,
    ) -> Result<Vec<u8>> {
        let ret = smt
            .merkle_proof(keys)
            .map_err(|e| MercuryError::SMTError(e.to_string()))?
            .compile(leaves)
            .map_err(|e| MercuryError::SMTError(e.to_string()))?;

        Ok(ret.into())
    }

    // TODO: deny reduplicate key in the update list now.
    fn update_smt(&self, smt: &mut SMT, update_items: &[SMTUpdateItem]) -> Result<()> {
        for item in update_items.iter() {
            smt.update(item.key.0.into(), build_smt_value(item.new_val == 1))
                .map_err(|e| MercuryError::SMTError(e.to_string()))?;
        }

        Ok(())
    }

    fn get_rc_rule(&self, data: &[u8]) -> xudt_rce::RCRule {
        let rc_data = xudt_rce::RCData::from_slice(data)
            .expect("invalid data format")
            .to_enum();

        match rc_data {
            xudt_rce::RCDataUnion::RCRule(rule) => rule,
            xudt_rce::RCDataUnion::RCCellVec(_cells) => unreachable!(),
        }
    }

    fn build_rce_transaction(
        &self,
        origin: packed::Transaction,
        index: usize,
        cell_data: Bytes,
        witness_args: Bytes,
    ) -> TransactionView {
        let mut witness = origin.witnesses().unpack();
        let mut output_data = origin.clone().into_view().outputs_data().unpack();
        change_witness(&mut witness, index, witness_args);
        swap_item(&mut output_data, index, cell_data);

        origin
            .as_advanced_builder()
            .witnesses(witness.pack())
            .outputs_data(output_data.pack())
            .build()
            .into()
    }

    fn build_rce_data(&self, root: [u8; 32], flag: packed::Byte) -> Bytes {
        xudt_rce::RCDataBuilder(xudt_rce::RCDataUnion::RCRule(
            xudt_rce::RCRuleBuilder::default()
                .flags(flag)
                .smt_root(root.pack())
                .build(),
        ))
        .build()
        .as_bytes()
    }

    fn build_witness_args(&self, proof: Vec<u8>, update_item: &[SMTUpdateItem]) -> Result<Bytes> {
        let update_inner = update_item
            .iter()
            .map(|item| {
                xudt_rce::SmtUpdateItemBuilder::default()
                    .key(item.key.pack())
                    .values(item.new_val.into())
                    .build()
            })
            .collect::<Vec<_>>();
        let update = xudt_rce::SmtUpdateVecBuilder(update_inner).build();
        let merkle_proof =
            xudt_rce::SmtProofBuilder(proof.into_iter().map(Into::into).collect()).build();

        Ok(xudt_rce::SmtUpdateBuilder::default()
            .proof(merkle_proof)
            .update(update)
            .build()
            .as_bytes())
    }

    fn parse_xudt_data(&self, data: Vec<Bytes>) -> Result<Vec<InnerRCRule>> {
        if data.is_empty() {
            return Err(MercuryError::MissingRCData.into());
        }

        if data.len() == 1 {
            // The RC data is only one RC rule.
            let raw_data = data.get(0).cloned().unwrap();
            let rule = xudt_rce::RCRule::from_slice(&raw_data).unwrap();
            let kind = RCState::from(rule.flags());
            let root: [u8; 32] = rule.smt_root().unpack();
            let smt = SMT::new(root.into(), Default::default());

            return Ok(vec![InnerRCRule::new(kind, smt)]);
        }

        // The RCCellVec can include at most 8196 RC rules.
        if data.len() > MAX_RCE_RULE_NUM {
            return Err(MercuryError::RCRuleNumOverMax(data.len()).into());
        }

        Ok(data
            .iter()
            .map(|raw_data| {
                let rule = xudt_rce::RCRule::from_slice(&raw_data).unwrap();
                let kind = RCState::from(rule.flags());
                let root: [u8; 32] = rule.smt_root().unpack();
                let smt = SMT::new(root.into(), Default::default());
                InnerRCRule::new(kind, smt)
            })
            .collect::<Vec<_>>())
    }

    fn check_rules(&self, hash_list: &[[u8; 32]], rule_list: &[InnerRCRule]) -> Result<Bytes> {
        let mut proof_list = Vec::new();

        for rule in rule_list.iter() {
            match rule.kind {
                RCState::WhiteList => {
                    self.check_white_list(hash_list, rule, &mut proof_list)?;
                }

                RCState::BlackList => {
                    self.check_black_list(hash_list, rule, &mut proof_list)?;
                }

                _ => {
                    return Err(MercuryError::RCRuleIsInStopState(hex::encode(
                        rule.smt.root().as_slice(),
                    ))
                    .into())
                }
            }
        }

        let mut ret_builder = xudt_rce::SmtProofVecBuilder::default();
        for proof in proof_list.iter() {
            ret_builder = ret_builder.push(
                xudt_rce::SmtProofBuilder::default()
                    .set(proof.clone())
                    .build(),
            );
        }

        Ok(ret_builder.build().as_bytes())
    }

    fn check_white_list(
        &self,
        hash_list: &[[u8; 32]],
        rule: &InnerRCRule,
        proof_list: &mut Vec<Vec<packed::Byte>>,
    ) -> Result<()> {
        let smt = &rule.smt;
        let mut keys: Vec<smt::H256> = Vec::new();
        let mut leaves: Vec<(smt::H256, smt::H256)> = Vec::new();

        for hash in hash_list.iter() {
            let tmp: [u8; 32] = *hash;
            let res = smt
                .get(&tmp.into())
                .map_err(|e| MercuryError::SMTError(e.to_string()))?;

            if res.is_zero() {
                return Err(MercuryError::CheckWhiteListFailed(hex::encode(tmp)).into());
            } else {
                keys.push(tmp.into());
                leaves.push((tmp.into(), res));
            }
        }

        let proof = self
            .build_merkle_proof(&smt, keys, leaves)?
            .iter()
            .map(|byte| packed::Byte::new(*byte))
            .collect::<Vec<_>>();

        proof_list.push(proof);
        Ok(())
    }

    fn check_black_list(
        &self,
        hash_list: &[[u8; 32]],
        rule: &InnerRCRule,
        proof_list: &mut Vec<Vec<packed::Byte>>,
    ) -> Result<()> {
        let smt = &rule.smt;
        let mut keys: Vec<smt::H256> = Vec::new();
        let mut leaves: Vec<(smt::H256, smt::H256)> = Vec::new();

        for hash in hash_list.iter() {
            let tmp: [u8; 32] = *hash;
            let res = smt
                .get(&tmp.into())
                .map_err(|e| MercuryError::SMTError(e.to_string()))?;

            if !res.is_zero() {
                return Err(MercuryError::CheckBlackListFailed(hex::encode(tmp)).into());
            } else {
                keys.push(tmp.into());
                leaves.push((tmp.into(), res));
            }
        }

        let proof = self
            .build_merkle_proof(&smt, keys, leaves)?
            .iter()
            .map(|byte| packed::Byte::new(*byte))
            .collect::<Vec<_>>();

        proof_list.push(proof);
        Ok(())
    }
}

fn build_smt_value(is_in: bool) -> smt::H256 {
    let v = is_in.into();
    let ret: [u8; 32] = array_init::array_init(|i| if i == 0 { v } else { 0 });
    ret.into()
}
