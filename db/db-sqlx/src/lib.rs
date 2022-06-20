use common::{anyhow::anyhow, Result};
use log::LevelFilter;
use once_cell::sync::OnceCell;
use protocol::db::DBDriver;
use sqlx::any::{Any, AnyArguments, AnyPool, AnyPoolOptions, AnyRow};
use sqlx::query::{Query, QueryAs};
use sqlx::{IntoArguments, Row, Transaction};

use std::marker::{Send, Unpin};
use std::{fmt::Debug, sync::Arc, time::Duration};

#[derive(Clone)]
pub struct SQLXPool {
    pool: Arc<OnceCell<AnyPool>>,
    center_id: u16,
    node_id: u16,
    max_conn: u32,
    min_conn: u32,
    conn_timeout: Duration,
    max_lifetime: Duration,
    idle_timeout: Duration,
}

impl Debug for SQLXPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLXPool")
            .field("center_id", &self.center_id)
            .field("node_id", &self.node_id)
            .field("max_conn", &self.max_conn)
            .field("min_conn", &self.min_conn)
            .field("conn_timeout", &self.conn_timeout)
            .field("max_lifetime", &self.max_lifetime)
            .field("idle_timeout", &self.idle_timeout)
            .finish()
    }
}

impl SQLXPool {
    pub fn new(
        center_id: u16,
        node_id: u16,
        max_connections: u32,
        min_connections: u32,
        connection_timeout: u64,
        max_lifetime: u64,
        idle_timeout: u64,
        _log_level: LevelFilter,
    ) -> Self {
        SQLXPool {
            pool: Arc::new(OnceCell::new()),
            center_id,
            node_id,
            max_conn: max_connections,
            min_conn: min_connections,
            conn_timeout: Duration::from_secs(connection_timeout),
            max_lifetime: Duration::from_secs(max_lifetime),
            idle_timeout: Duration::from_secs(idle_timeout),
        }
    }

    pub async fn connect(
        &mut self,
        db_driver: &DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        let pool_options = AnyPoolOptions::new()
            .max_connections(self.max_conn)
            .min_connections(self.min_conn)
            .connect_timeout(self.conn_timeout)
            .max_lifetime(self.max_lifetime)
            .idle_timeout(self.idle_timeout);
        let uri = build_url(db_driver.into(), db_name, host, port, user, password);
        let pool = pool_options.connect(&uri).await?;
        self.pool
            .set(pool)
            .map_err(|_| anyhow!("set pg pool failed!"))
    }

    pub async fn fetch_count(&self, table_name: &str) -> Result<u64> {
        let pool = self.get_pool()?;
        let row = sqlx::query(&["SELECT COUNT(*) as number FROM ", table_name].join(""))
            .fetch_one(pool)
            .await?;
        let count: i64 = row.get::<i64, _>("number");
        Ok(count.try_into().expect("i64 to u64"))
    }

    pub fn new_query(sql: &str) -> Query<Any, AnyArguments> {
        sqlx::query(sql)
    }

    pub fn new_query_as<T>(sql: &str) -> QueryAs<Any, T, AnyArguments>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow>,
    {
        sqlx::query_as(sql)
    }

    pub async fn fetch_one<'a, T>(&self, query: Query<'a, Any, T>) -> Result<AnyRow>
    where
        T: Send + IntoArguments<'a, Any> + 'a,
    {
        let pool = self.get_pool()?;
        let r = query.fetch_one(pool).await?;
        Ok(r)
    }

    pub async fn fetch_one_by_query_as<T>(
        &self,
        query: QueryAs<'static, Any, T, AnyArguments<'static>>,
    ) -> Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, AnyRow> + Unpin + Send,
    {
        let pool = self.get_pool()?;
        let t = query.fetch_one(pool).await?;
        Ok(t)
    }

    pub async fn transaction(&self) -> Result<Transaction<'_, Any>> {
        let pool = self.get_pool()?;
        let tx = pool.begin().await?;
        Ok(tx)
    }

    fn get_pool(&self) -> Result<&AnyPool> {
        let pool = self.pool.get();
        if pool.is_none() {
            return Err(anyhow!("pg pool not inited!"));
        }
        Ok(pool.unwrap())
    }

    pub fn center_id(&self) -> u16 {
        self.center_id
    }

    pub fn node_id(&self) -> u16 {
        self.node_id
    }

    pub fn get_max_connections(&self) -> u32 {
        self.max_conn
    }
}

fn build_url(
    db_type: &str,
    db_name: &str,
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> String {
    if db_type == protocol::db::SQLITE {
        return db_type.to_string() + db_name;
    }

    db_type.to_string()
        + user
        + ":"
        + password
        + "@"
        + host
        + ":"
        + port.to_string().as_str()
        + "/"
        + db_name
}
