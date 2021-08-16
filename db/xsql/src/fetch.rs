use crate::{
    error::DBError,
    table::{BlockTable, UncleRelationshipTable},
    DBAdapter, XSQLPool,
};

use common::{anyhow::Result, utils};

use ckb_types::{
    core::{
        BlockBuilder, BlockNumber, BlockView, EpochNumberWithFraction, HeaderBuilder, HeaderView,
        UncleBlockView,
    },
    packed,
    prelude::*,
    H256,
};

use rbatis::crud::CRUD;

use std::str;

impl<T: DBAdapter> XSQLPool<T> {
    pub async fn get_block_by_number(&self, block_number: BlockNumber) -> Result<BlockView> {
        let block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_number", &block_number)
            .await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::WrongHeight.into()),
        };
        self.get_block_view(&block).await
    }

    pub async fn get_block_by_hash(&self, block_hash: H256) -> Result<BlockView> {
        let block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_hash", &block_hash)
            .await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::CannotFind.into()),
        };
        self.get_block_view(&block).await
    }

    pub async fn get_tip_block(&self) -> Result<BlockView> {
        let wrapper = self.wrapper().order_by(false, &["block_number"]).limit(1);
        let block: Option<BlockTable> = self.inner.fetch_by_wrapper(&wrapper).await?;
        let block = block.expect("get tip block");
        self.get_block_view(&block).await
    }

    pub async fn get_tip_block_header(&self) -> Result<HeaderView> {
        let wrapper = self.wrapper().order_by(false, &["block_number"]).limit(1);
        let block: Option<BlockTable> = self.inner.fetch_by_wrapper(&wrapper).await?;
        Ok(self.get_header_view(&block.expect("get tip block")))
    }

    pub async fn get_block_header_by_block_hash(&self, block_hash: H256) -> Result<HeaderView> {
        let block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_hash", &block_hash)
            .await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::CannotFind.into()),
        };
        Ok(self.get_header_view(&block))
    }

    pub async fn get_block_header_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<HeaderView> {
        let block: Option<BlockTable> = self
            .inner
            .fetch_by_column("block_number", &block_number)
            .await?;
        let block = match block {
            Some(block) => block,
            None => return Err(DBError::WrongHeight.into()),
        };
        Ok(self.get_header_view(&block))
    }

    async fn get_block_view(&self, block: &BlockTable) -> Result<BlockView> {
        let header = self.get_header_view(&block);
        let uncles = self.get_uncle_block_views(&block).await;
        // let txs = get_transactions(&block);
        // let proposals = get_proposals(&block);
        let block_view = BlockBuilder::default()
            .header(header)
            .uncles(uncles)
            .build();
        Ok(block_view)
    }

    async fn get_uncle_block_views(&self, block: &BlockTable) -> Vec<UncleBlockView> {
        let uncles: Vec<UncleRelationshipTable> = self
            .inner
            .fetch_list_by_column("block_hash", &[self.convert_bytea(&block.block_hash)])
            .await
            .expect("fetch uncle block hash");
        let uncle_hashes: Vec<Vec<u8>> = uncles
            .iter()
            .map(|uncle| self.convert_bytea(&uncle.uncles_hash))
            .collect();
        let uncles: Vec<BlockTable> = self
            .inner
            .fetch_list_by_column("block_hash", &uncle_hashes)
            .await
            .expect("fetch uncle block");
        uncles.into_iter().map(|_| todo!()).collect()
    }

    fn get_header_view(&self, block: &BlockTable) -> HeaderView {
        HeaderBuilder::default()
            .number(block.block_number.pack())
            .parent_hash(
                packed::Byte32::from_slice(&self.convert_bytea(&block.parent_hash))
                    .expect("impossible: fail to pack parent_hash"),
            )
            .compact_target(block.compact_target.pack())
            .nonce(utils::decode_nonce(&block.nonce).pack())
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
            .dao(packed::Byte32::from_slice(&self.convert_bytea(&block.dao)).expect(""))
            .transactions_root(
                packed::Byte32::from_slice(&self.convert_bytea(&block.transactions_root))
                    .expect("impossible: fail to pack transactions_root"),
            )
            .proposals_hash(
                packed::Byte32::from_slice(&self.convert_bytea(&block.proposals_hash))
                    .expect("impossible: fail to pack proposals_hash"),
            )
            .uncles_hash(
                packed::Byte32::from_slice(&self.convert_bytea(&block.uncles_hash))
                    .expect("impossible: fail to pack uncles_hash"),
            )
            .build()
    }

    fn convert_bytea(&self, input: &Vec<u8>) -> Vec<u8> {
        let input: molecule::bytes::Bytes = molecule::bytes::Bytes::from(input.clone());
        let input = str::from_utf8(&input).unwrap();
        let pattern: &[_] = &['[', ']'];
        let input = input.trim_matches(pattern);
        let input: Vec<&str> = input.split(',').collect();
        input.iter().map(|&c| c.parse().unwrap()).collect()
    }
}
