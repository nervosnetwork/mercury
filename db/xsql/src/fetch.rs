use crate::table::{BlockTable, CellTable, TransactionTable, UncleRelationshipTable};
use crate::{error::DBError, DBAdapter, XSQLPool};

use common::{anyhow::Result, utils};

use bson::Binary;
use ckb_types::bytes::Bytes;
use ckb_types::core::{
    BlockBuilder, BlockNumber, BlockView, EpochNumberWithFraction, HeaderBuilder, HeaderView,
    TransactionBuilder, TransactionView, UncleBlockView,
};
use ckb_types::packed::{
    Byte32, CellDep, CellInput, CellOutput, ProposalShortIdVec, UncleBlockBuilder,
};
use ckb_types::{packed, prelude::*, H256};
use rbatis::crud::CRUD;

use std::collections::HashMap;

pub type BsonBytes = Binary;

impl<T: DBAdapter> XSQLPool<T> {
    pub async fn get_block_by_number(&self, block_number: BlockNumber) -> Result<BlockView> {
        let block = self.query_block_by_number(block_number).await?;
        self.get_block_view(&block).await
    }

    pub async fn get_block_by_hash(&self, block_hash: H256) -> Result<BlockView> {
        let block = self.query_block_by_hash(block_hash).await?;
        self.get_block_view(&block).await
    }

    pub async fn get_tip_block(&self) -> Result<BlockView> {
        let block = self.query_tip_block().await?;
        self.get_block_view(&block).await
    }

    pub async fn get_tip_block_header(&self) -> Result<HeaderView> {
        let block = self.query_tip_block().await?;
        Ok(build_header_view(&block))
    }

    pub async fn get_block_header_by_block_hash(&self, block_hash: H256) -> Result<HeaderView> {
        let block = self.query_block_by_hash(block_hash).await?;
        Ok(build_header_view(&block))
    }

    pub async fn get_block_header_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<HeaderView> {
        let block = self.query_block_by_number(block_number).await?;
        Ok(build_header_view(&block))
    }

    async fn get_block_view(&self, block: &BlockTable) -> Result<BlockView> {
        let header = build_header_view(&block);
        let uncles = self.get_uncle_block_views(&block).await?;
        let txs = self.get_transactions(&block).await?;
        let proposals = build_proposals(&block.proposals.bytes);
        Ok(build_block_view(header, uncles, txs, proposals))
    }

    async fn get_uncle_block_views(&self, block: &BlockTable) -> Result<Vec<UncleBlockView>> {
        let uncles = self.query_uncles_by_hash(&block.block_hash).await?;
        let uncles: Vec<UncleBlockView> = uncles
            .into_iter()
            .map(|uncle| build_uncle_block_view(&uncle))
            .collect();
        Ok(uncles)
    }

    async fn get_transactions(&self, block: &BlockTable) -> Result<Vec<TransactionView>> {
        let txs = self.query_transactions(&block.block_hash).await?;
        let tx_hashes: Vec<Binary> = txs.iter().map(|tx| tx.tx_hash.clone()).collect();
        let cells: Vec<CellTable> = self.query_txs_cells(&tx_hashes).await?;

        let mut txs_input_cells: HashMap<Vec<u8>, Vec<CellTable>> = txs
            .iter()
            .map(|tx| (tx.tx_hash.bytes.clone(), vec![]))
            .collect();
        let mut txs_output_cells: HashMap<Vec<u8>, Vec<CellTable>> = txs
            .iter()
            .map(|tx| (tx.tx_hash.bytes.clone(), vec![]))
            .collect();
        for cell in cells {
            match cell.input_index {
                Some(_) => {
                    if let Some(set) = txs_input_cells.get_mut(&cell.tx_hash.bytes) {
                        (*set).push(cell)
                    }
                }
                None => {
                    if let Some(set) = txs_output_cells.get_mut(&cell.tx_hash.bytes) {
                        (*set).push(cell)
                    }
                }
            }
        }

        let tx_views = txs
            .into_iter()
            .map(|tx| {
                let witness = build_witness(&tx.witnesses.bytes);
                let header_deps = build_header_deps(&tx.header_deps.bytes);
                let cell_deps = build_cell_deps(&tx.cell_deps.bytes);
                let inputs = build_cell_inputs(txs_input_cells.get(&tx.block_hash.bytes));
                let outputs = build_cell_outputs(txs_output_cells.get(&tx.block_hash.bytes));
                let outputs_data = build_outputs_data(txs_output_cells.get(&tx.block_hash.bytes));
                build_transaction_view(
                    tx.version as u32,
                    witness,
                    inputs,
                    outputs,
                    outputs_data,
                    cell_deps,
                    header_deps,
                )
            })
            .collect();
        Ok(tx_views)
    }

    // TODO: query refactoring
    async fn query_tip_block(&self) -> Result<BlockTable> {
        let wrapper = self.wrapper().order_by(false, &["block_number"]).limit(1);
        let block: Option<BlockTable> = self.inner.fetch_by_wrapper(&wrapper).await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::CannotFind.into()),
        };
        Ok(block)
    }

    async fn query_block_by_hash(&self, block_hash: H256) -> Result<BlockTable> {
        let block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_hash", &block_hash)
            .await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::CannotFind.into()),
        };
        Ok(block)
    }

    async fn query_block_by_number(&self, block_number: BlockNumber) -> Result<BlockTable> {
        let block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_number", &block_number)
            .await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::WrongHeight.into()),
        };
        Ok(block)
    }

    async fn query_uncles_by_hash(&self, block_hash: &Binary) -> Result<Vec<BlockTable>> {
        let uncle_relationships: Vec<UncleRelationshipTable> = self
            .inner
            .fetch_list_by_column("block_hash", &[block_hash])
            .await?;
        let uncle_hashes: Vec<&BsonBytes> = uncle_relationships
            .iter()
            .map(|uncle_relationship| &uncle_relationship.uncle_hashes)
            .collect();
        let uncles: Vec<BlockTable> = self
            .inner
            .fetch_list_by_column("block_hash", &uncle_hashes)
            .await?;
        Ok(uncles)
    }

    async fn query_transactions(&self, block_hash: &Binary) -> Result<Vec<TransactionTable>> {
        let txs: Vec<TransactionTable> = self
            .inner
            .fetch_list_by_column("block_hash", &[block_hash])
            .await?;
        Ok(txs)
    }

    async fn query_txs_cells(&self, tx_hashes: &[Binary]) -> Result<Vec<CellTable>> {
        let cells: Vec<CellTable> = self
            .inner
            .fetch_list_by_column("tx_hash", &[tx_hashes])
            .await?;
        Ok(cells)
    }
}

fn build_block_view(
    header: HeaderView,
    uncles: Vec<UncleBlockView>,
    txs: Vec<TransactionView>,
    proposals: ProposalShortIdVec,
) -> BlockView {
    BlockBuilder::default()
        .header(header)
        .uncles(uncles)
        .transactions(txs)
        .proposals(proposals)
        .build()
}

fn build_uncle_block_view(block: &BlockTable) -> UncleBlockView {
    UncleBlockBuilder::default()
        .header(build_header_view(&block).data())
        .proposals(build_proposals(&block.proposals.bytes))
        .build()
        .into_view()
}

fn build_header_view(block: &BlockTable) -> HeaderView {
    HeaderBuilder::default()
        .number(block.block_number.pack())
        .parent_hash(
            packed::Byte32::from_slice(&block.parent_hash.bytes)
                .expect("impossible: fail to pack parent_hash"),
        )
        .compact_target(block.compact_target.pack())
        .nonce(utils::decode_nonce(&block.nonce.bytes).pack())
        .timestamp(block.block_timestamp.pack())
        .version((block.version as u32).pack())
        .epoch(
            EpochNumberWithFraction::new(
                block.epoch_number,
                block.epoch_block_index as u64,
                block.epoch_length as u64,
            )
            .number()
            .pack(),
        )
        .dao(packed::Byte32::from_slice(&block.dao.bytes).expect("impossible: fail to pack dao"))
        .transactions_root(
            packed::Byte32::from_slice(&block.transactions_root.bytes)
                .expect("impossible: fail to pack transactions_root"),
        )
        .proposals_hash(
            packed::Byte32::from_slice(&block.proposals_hash.bytes)
                .expect("impossible: fail to pack proposals_hash"),
        )
        .uncles_hash(
            packed::Byte32::from_slice(&block.uncles_hash.bytes)
                .expect("impossible: fail to pack uncles_hash"),
        )
        .build()
}

fn build_proposals(_input: &[u8]) -> ProposalShortIdVec {
    todo!()
}

fn build_witness(_input: &[u8]) -> Vec<packed::Bytes> {
    todo!()
}

fn build_header_deps(_input: &[u8]) -> Vec<Byte32> {
    todo!()
}

fn build_cell_deps(_input: &[u8]) -> Vec<CellDep> {
    todo!()
}

fn build_cell_inputs(_inputs: Option<&Vec<CellTable>>) -> Vec<CellInput> {
    todo!()
}

fn build_cell_outputs(_inputs: Option<&Vec<CellTable>>) -> Vec<CellOutput> {
    todo!()
}

fn build_outputs_data(cells: Option<&Vec<CellTable>>) -> Vec<packed::Bytes> {
    let cells = match cells {
        Some(cells) => cells,
        None => return vec![],
    };
    cells
        .into_iter()
        .map(|cell| Bytes::from(cell.data.bytes.clone()).pack())
        .collect()
}

fn build_transaction_view(
    version: u32,
    witnesses: Vec<packed::Bytes>,
    inputs: Vec<CellInput>,
    outputs: Vec<CellOutput>,
    outputs_data: Vec<packed::Bytes>,
    cell_deps: Vec<CellDep>,
    header_deps: Vec<packed::Byte32>,
) -> TransactionView {
    TransactionBuilder::default()
        .version(version.pack())
        .witnesses(witnesses)
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data)
        .cell_deps(cell_deps)
        .header_deps(header_deps)
        .build()
}
