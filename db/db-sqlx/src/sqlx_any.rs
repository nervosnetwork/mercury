use common::{anyhow::anyhow, Result};
use once_cell::sync::OnceCell;
use sqlx::any::{Any, AnyPool, AnyPoolOptions};
use sqlx::{pool::PoolConnection, Transaction};

pub struct PgSqlx {
    pub pool: OnceCell<AnyPool>,
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

    pub async fn link_opt(&self, uri: &str, pool_options: AnyPoolOptions) -> Result<()> {
        let pool = pool_options.connect(uri).await?;
        self.pool
            .set(pool)
            .map_err(|_| anyhow!("set pg pool failed!"))
    }

    pub fn get_pool(&self) -> Result<&AnyPool> {
        let pool = self.pool.get();
        if pool.is_none() {
            return Err(anyhow!("pg pool not inited!"));
        }
        Ok(pool.unwrap())
    }

    pub async fn acquire_begin(&self) -> Result<Transaction<'_, Any>> {
        let pool = self.get_pool()?;
        let tx = pool.begin().await?;
        Ok(tx)
    }

    pub async fn _acquire(&self) -> Result<PoolConnection<Any>> {
        let pool = self.get_pool()?;
        let conn = pool.acquire().await?;
        Ok(conn)
    }
}
