pub mod page;

pub use rbatis;

use common::Result;
use db_protocol::DBDriver;

use log::LevelFilter;
use rbatis::crud::{CRUDTable, CRUD};
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::{
    core::db::DBPoolOptions, plugin::log::RbatisLogPlugin, rbatis::Rbatis, wrapper::Wrapper,
};
use serde::{de::DeserializeOwned, ser::Serialize};

use std::{fmt::Debug, sync::Arc};
use std::time::Duration;

#[derive(Clone)]
pub struct XSQLPool {
    pool: Arc<Rbatis>,
    center_id: u16,
    node_id: u16,
    config: DBPoolOptions,
}

impl Debug for XSQLPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XSQLPool")
            .field("center_id", &self.center_id)
            .field("node_id", &self.node_id)
            .field("config", &self.config)
            .finish()
    }
}

impl XSQLPool {
    pub fn new(max_connections: u32, center_id: u16, node_id: u16, log_level: LevelFilter) -> Self {
        let config = DBPoolOptions {
            max_connections,
            idle_timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        };

        let mut rbatis = Rbatis::new();
        rbatis.set_log_plugin(RbatisLogPlugin {
            level_filter: log_level,
        });
        rbatis.set_page_plugin(page::CursorPagePlugin);

        XSQLPool {
            pool: Arc::new(rbatis),
            center_id,
            node_id,
            config,
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
                &self.config,
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

    pub fn get_config(&self) -> DBPoolOptions {
        self.config
    }

    pub fn center_id(&self) -> u16 {
        self.center_id
    }

    pub fn node_id(&self) -> u16 {
        self.node_id
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
    if db_type == db_protocol::SQLITE {
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
