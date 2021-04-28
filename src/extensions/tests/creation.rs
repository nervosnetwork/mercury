use crate::extensions::tests::{HashType, SUDT_CODE_HASH};
use crate::utils::to_fixed_array;

use ckb_types::bytes::Bytes;
use ckb_types::core::{BlockNumber, BlockView, TransactionBuilder, TransactionView};
use ckb_types::{packed, prelude::*};

fn create_ckb_script(args: Bytes, hash_type: HashType) -> packed::Script {
    packed::Script::new_builder()
        .args(args.pack())
        .hash_type(hash_type.into())
        .build()
}

pub fn create_sudt_script(args: Bytes) -> packed::Script {
    let code_hash = hex::decode(SUDT_CODE_HASH).unwrap();

    packed::Script::new_builder()
        .args(args.pack())
        .hash_type(HashType::Data.into())
        .code_hash(to_fixed_array(&code_hash).pack())
        .build()
}

pub fn create_ckb_cell(lock_args: Bytes, capacity: u64) -> packed::CellOutput {
    packed::CellOutput::new_builder()
        .lock(create_ckb_script(lock_args, HashType::Data))
        .capacity(capacity.pack())
        .build()
}

pub fn create_sudt_cell(lock_args: Bytes, sudt_args: Bytes, capacity: u64) -> packed::CellOutput {
    packed::CellOutput::new_builder()
        .lock(create_ckb_script(lock_args, HashType::Data))
        .type_(Some(create_sudt_script(sudt_args)).pack())
        .capacity(capacity.pack())
        .build()
}

pub fn create_input_cell(
    out_point: packed::OutPoint,
    block_number: BlockNumber,
) -> packed::CellInput {
    packed::CellInput::new(out_point, block_number)
}

pub fn default_data_list(len: usize) -> Vec<packed::Bytes> {
    (0..len).map(|_| Default::default()).collect::<Vec<_>>()
}

pub fn default_witness_list(len: usize) -> Vec<packed::Bytes> {
    (0..len).map(|_| Default::default()).collect::<Vec<_>>()
}

pub fn create_transaction(
    inputs: Vec<packed::CellInput>,
    outputs: Vec<packed::CellOutput>,
    outputs_data: Vec<packed::Bytes>,
    witnesses: Vec<packed::Bytes>,
) -> TransactionView {
    TransactionBuilder::default()
        .set_inputs(inputs)
        .set_outputs(outputs)
        .set_witnesses(witnesses)
        .outputs_data(outputs_data)
        .build()
}

pub fn create_block(
    number: BlockNumber,
    epoch: u64,
    transactions: Vec<TransactionView>,
) -> BlockView {
    packed::BlockBuilder::default()
        .build()
        .into_view()
        .as_advanced_builder()
        .number(number.pack())
        .epoch(epoch.pack())
        .transactions(transactions)
        .build()
}
