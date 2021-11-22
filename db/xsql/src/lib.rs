pub mod page;

pub use rbatis;

use common::{anyhow::anyhow, Result};
use protocol::db::DBDriver;

use log::LevelFilter;
use rbatis::crud::{CRUDTable, CRUD};
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::{
    core::db::DBPoolOptions, plugin::log::RbatisLogPlugin, rbatis::Rbatis, wrapper::Wrapper,
};
use serde::{de::DeserializeOwned, ser::Serialize};

use std::time::Duration;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct XSQLPool {
    pool: Arc<Rbatis>,
    center_id: u16,
    node_id: u16,
    max_conn: u32,
}

impl Debug for XSQLPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XSQLPool")
            .field("center_id", &self.center_id)
            .field("node_id", &self.node_id)
            .field("max_conn", &self.max_conn)
            .finish()
    }
}

impl XSQLPool {
    pub fn new(max_conn: u32, center_id: u16, node_id: u16, log_level: LevelFilter) -> Self {
        let mut rbatis = Rbatis::new();
        rbatis.set_log_plugin(RbatisLogPlugin {
            level_filter: log_level,
        });
        rbatis.set_page_plugin(page::CursorPagePlugin);

        XSQLPool {
            pool: Arc::new(rbatis),
            center_id,
            node_id,
            max_conn,
        }
    }

    pub async fn connect(
        &self,
        db_driver: DBDriver,
        db_name: &str,
        host: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.pool
            .link_opt(
                &build_url(db_driver.into(), db_name, host, port, user, password),
                DBPoolOptions {
                    max_connections: self.max_conn,
                    min_connections: 2,
                    idle_timeout: Some(Duration::from_secs(3)),
                    test_before_acquire: false,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        Ok(())
    }

    pub async fn transaction(&self) -> Result<RBatisTxExecutor<'_>> {
        let tx = self.pool.acquire_begin().await?;
        Ok(tx)
    }

    pub async fn acquire(&self) -> Result<RBatisConnExecutor<'_>> {
        let conn = self.pool.acquire().await?;
        Ok(conn)
    }

    pub async fn fetch_count_by_wrapper<T: CRUDTable>(&self, w: Wrapper) -> Result<u64> {
        let ret = self.pool.fetch_count_by_wrapper::<T>(w).await?;
        Ok(ret)
    }

    pub async fn fetch_by_wrapper<T: CRUDTable + DeserializeOwned>(&self, w: Wrapper) -> Result<T> {
        let ret = self.pool.fetch_by_wrapper(w).await?;
        Ok(ret)
    }

    pub async fn fetch_list_by_wrapper<T: CRUDTable + DeserializeOwned>(
        &self,
        w: Wrapper,
    ) -> Result<Vec<T>> {
        let ret = self.pool.fetch_list_by_wrapper(w).await?;
        Ok(ret)
    }

    pub async fn fetch_by_column<T: CRUDTable + DeserializeOwned, C: Serialize + Sync + Send>(
        &self,
        column: &str,
        value: &C,
    ) -> Result<T> {
        let ret = self.pool.fetch_by_column(column, value).await?;
        Ok(ret)
    }

    pub async fn fetch_list_by_column<
        T: CRUDTable + DeserializeOwned,
        C: Serialize + Sync + Send,
    >(
        &self,
        column: &str,
        values: &[C],
    ) -> Result<Vec<T>> {
        let ret = self.pool.fetch_list_by_column(column, values).await?;
        Ok(ret)
    }

    pub async fn fetch_list<T: CRUDTable + DeserializeOwned>(&self) -> Result<Vec<T>> {
        let ret = self.pool.fetch_list().await?;
        Ok(ret)
    }

    pub fn wrapper(&self) -> Wrapper {
        self.pool.new_wrapper()
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

pub async fn commit_transaction(mut tx: RBatisTxExecutor<'_>) -> Result<()> {
    if tx.commit().await.is_err() {
        tx.rollback().await?;
        return Err(anyhow!("Commit transaction failed, transaction rollback!"));
    }

    Ok(())
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
