mod page;

pub use rbatis;

use common::anyhow::Result;
use db_protocol::DBDriver;

use log::LevelFilter;
use rbatis::executor::{RBatisConnExecutor, RBatisTxExecutor};
use rbatis::{
    core::db::DBPoolOptions, plugin::log::RbatisLogPlugin, rbatis::Rbatis, wrapper::Wrapper,
};

use std::sync::Arc;

#[derive(Clone)]
pub struct XSQLPool {
    pool: Arc<Rbatis>,
    center_id: u16,
    node_id: u16,
    config: DBPoolOptions,
}

impl XSQLPool {
    pub fn new(max_connections: u32, center_id: u16, node_id: u16, log_level: LevelFilter) -> Self {
        let config = DBPoolOptions {
            max_connections,
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

    pub fn wrapper(&self) -> Wrapper {
        self.pool.new_wrapper()
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
