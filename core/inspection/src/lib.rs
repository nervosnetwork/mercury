use common::Result;
use protocol::storage::StorageCheck;

pub struct MercuryInspection<SC> {
    storage: SC,
}

impl<SC: StorageCheck> MercuryInspection<SC> {
    pub fn new(storage: SC) -> Self {
        MercuryInspection { storage }
    }

    pub async fn check(&self) -> Result<()> {
        self.check_count().await?;
        self.chech_redupicate_tx().await?;
        self.chech_redupicate_cell().await?;
        Ok(())
    }

    async fn check_count(&self) -> Result<()> {
        let cell_count = self.storage.get_cell_table_consumed_null_count().await?;
        let live_cell_count = self.storage.get_live_cell_table_count().await?;

        if cell_count != live_cell_count {
            println!(
                "[ERROR] consumed null count in cell {} mismatch live cell count {}!",
                cell_count, live_cell_count
            );
        }
        println!("/n");

        Ok(())
    }

    async fn chech_redupicate_tx(&self) -> Result<()> {
        let txs = self.storage.has_redupicate_txs().await?;

        if txs.is_empty() {
            return Ok(());
        }

        println!("[ERROR] exist redupicate transactions!");
        for hash in txs.iter() {
            println!("redupicate tx hash 0x{:?}", hex::encode(&hash.0));
        }
        println!("/n");

        Ok(())
    }

    async fn chech_redupicate_cell(&self) -> Result<()> {
        let cells = self.storage.has_redupicate_cells().await?;

        if cells.is_empty() {
            return Ok(());
        }

        println!("[ERROR] exist redupicate cells!");
        for cell in cells.iter() {
            println!(
                "redupicate cell outpoint tx_hash 0x{:?} index {:?}",
                hex::encode(&cell.tx_hash().raw_data()),
                cell.index()
            );
        }
        println!("/n");

        Ok(())
    }
}
