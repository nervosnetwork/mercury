use crate::PaginationRequest;

use common::Order;

use rbatis::plugin::page::{IPageRequest, PagePlugin};
use rbatis::{core::Error as RbError, sql::TEMPLATE, DriverType};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug)]
pub struct CursorPagePlugin;

impl PagePlugin for CursorPagePlugin {
    fn make_page_sql(
        &self,
        _dtype: &DriverType,
        sql: &str,
        _args: &Vec<bson::Bson>,
        page: &dyn IPageRequest,
    ) -> Result<(String, String), RbError> {
        debug_assert!(page.is_search_count());

        let mut sql = sql.trim().to_owned();
        if !sql.starts_with(TEMPLATE.select.right_space)
            && !sql.contains(TEMPLATE.from.left_right_space)
        {
            return Err(RbError::from("sql must contains 'select ' And ' from '"));
        }

        let mut count_sql = sql.clone();
        if page.is_search_count() {
            // make count sql
            count_sql = self.make_count_sql(&count_sql);
        }

        let page_part = format!("{} id > {}", TEMPLATE.r#where.value, page.get_page_no());

        let mut order_by_part = format!("{} id ", TEMPLATE.order_by.value);
        if !page.is_asc_order().unwrap() {
            order_by_part += TEMPLATE.desc.value;
        };

        let limit_part = format!(
            "{} {} {} {}",
            TEMPLATE.limit.value,
            page.get_page_size() + 1,
            TEMPLATE.offset.value,
            page.offset(),
        );

        let limit_sql = format!(
            "{} RB_DATA.*, ({})RB_DATA {} {} {}",
            TEMPLATE.select.value, sql, page_part, order_by_part, limit_part
        );

        sql += limit_sql.as_str();

        Ok((count_sql, sql))
    }
}

impl CursorPagePlugin {
    fn make_count_sql(&self, sql: &str) -> String {
        let mut from_index = sql.find(TEMPLATE.from.left_right_space);
        if from_index.is_some() {
            from_index = Option::Some(from_index.unwrap() + TEMPLATE.from.left_right_space.len());
        }
        let mut where_sql = sql[from_index.unwrap_or(0)..sql.len()].to_string();

        // Remove ORDER_BY.
        if where_sql.contains(TEMPLATE.order_by.left_right_space) {
            where_sql = where_sql[0..where_sql
                .rfind(TEMPLATE.order_by.left_right_space)
                .unwrap_or_else(|| where_sql.len())]
                .to_string();
        }

        // // Remove LIMIT.
        // if where_sql.contains(TEMPLATE.limit.left_right_space) {
        //     where_sql = where_sql[0..where_sql
        //         .rfind(TEMPLATE.limit.left_right_space)
        //         .unwrap_or_else(|| where_sql.len())]
        //         .to_string();
        // }

        format!("{} count(1) FROM {} ", TEMPLATE.select.value, where_sql)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct PageRequest {
    pub cursor: i64,
    pub skip: u64,
    pub count: u64,
    pub is_asc: bool,
    pub search_count: bool,
}

impl From<PaginationRequest> for PageRequest {
    fn from(p: PaginationRequest) -> Self {
        PageRequest {
            cursor: p.cursor,
            count: p.limit.unwrap_or(u64::MAX - 1),
            skip: p.skip.unwrap_or(0),
            is_asc: p.order == Order::Asc,
            search_count: true,
        }
    }
}

impl IPageRequest for PageRequest {
    fn get_page_size(&self) -> u64 {
        self.count
    }

    fn get_page_no(&self) -> u64 {
        self.cursor as u64
    }

    fn offset(&self) -> u64 {
        self.skip
    }

    fn is_asc_order(&self) -> Option<bool> {
        Some(self.is_asc)
    }

    fn is_search_count(&self) -> bool {
        self.search_count
    }

    fn get_total(&self) -> u64 {
        1u64
    }

    fn set_page_size(&mut self, arg: u64) {
        self.count = arg;
    }

    fn set_search_count(&mut self, arg: bool) {
        self.search_count = arg;
    }

    fn set_total(&mut self, _: u64) {}

    fn set_page_no(&mut self, _: u64) {}
}
