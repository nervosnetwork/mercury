use common::{anyhow::anyhow, Result};
use once_cell::sync::OnceCell;
use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, PgPool, Postgres, Transaction};

pub struct PgSqlx {
    pub pool: OnceCell<PgPool>,
}

impl Default for PgSqlx {
    fn default() -> PgSqlx {
        PgSqlx::new()
    }
}

impl PgSqlx {
    pub fn new() -> Self {
        Self::new_with_opt()
    }

    pub fn new_with_opt() -> Self {
        Self {
            pool: OnceCell::new(),
        }
    }

    pub async fn link_opt(&self, uri: &str, pool_options: PgPoolOptions) -> Result<()> {
        if uri.is_empty() {
            return Err(anyhow!("link url is empty!"));
        }
        let pool = pool_options.connect(uri).await?;
        self.pool
            .set(pool)
            .map_err(|_| anyhow!("set pg pool failed!"))
    }

    pub fn get_pool(&self) -> Result<&PgPool> {
        let pool = self.pool.get();
        if pool.is_none() {
            return Err(anyhow!("pg pool not inited!"));
        }
        Ok(pool.unwrap())
    }

    pub async fn acquire(&self) -> Result<PoolConnection<Postgres>> {
        let pool = self.get_pool()?;
        let conn = pool.acquire().await?;
        Ok(conn)
    }

    pub async fn acquire_begin(&self) -> Result<Transaction<'_, Postgres>> {
        let pool = self.get_pool()?;
        let tx = pool.begin().await?;
        Ok(tx)
    }
}
