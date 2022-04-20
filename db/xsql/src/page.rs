use common::PaginationRequest;

use rbatis::plugin::page::{IPageRequest, PagePlugin};
use rbatis::{core::Error as RbError, sql::TEMPLATE, DriverType};
use rbson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug)]
pub struct CursorPagePlugin;

impl PagePlugin for CursorPagePlugin {
    fn make_page_sql(
        &self,
        _dtype: &DriverType,
        sql: &str,
        _args: &Vec<Bson>,
        page: &dyn IPageRequest,
    ) -> Result<(String, String), RbError> {
        let sql = sql.trim().to_owned();
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

        let (first_part, second_part, has_where) = self.split_sql(&sql);

        let compare = if page.get_total() == 1 {
            String::from(">")
        } else {
            String::from("<")
        };

        let page_part = if has_where {
            format!(
                "id {} {} {}",
                compare,
                page.get_page_no(),
                TEMPLATE.and.value
            )
        } else {
            format!(
                "{} id {} {}",
                TEMPLATE.r#where.value,
                compare,
                page.get_page_no()
            )
        };

        let mut order_by_part = format!("{} id ", TEMPLATE.order_by.value);
        if page.get_total() == 1 {
            order_by_part += TEMPLATE.asc.value;
        } else {
            order_by_part += TEMPLATE.desc.value;
        };

        let limit_part = format!(
            "{} {} {} {}",
            TEMPLATE.limit.value,
            page.get_page_size(),
            TEMPLATE.offset.value,
            page.offset(),
        );

        let limit_sql = format!(
            "{} {} {} {} {}",
            first_part, page_part, second_part, order_by_part, limit_part
        );

        Ok((count_sql, limit_sql))
    }
}

impl CursorPagePlugin {
    fn split_sql(&self, sql: &str) -> (String, String, bool) {
        let (mid, has_where) = if sql.contains(TEMPLATE.r#where.left_right_space) {
            (
                sql.find(TEMPLATE.r#where.left_right_space).unwrap() + 6,
                true,
            )
        } else {
            (sql.len(), false)
        };
        let (a, b) = sql.split_at(mid);

        (a.to_string(), b.to_string(), has_where)
    }

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
    pub cursor: u64,
    pub skip: u64,
    pub count: u64,
    pub is_asc: bool,
    pub search_count: bool,
}

impl From<PaginationRequest> for PageRequest {
    // The id type in the table definition is i64,
    // so it is necessary to limit the numerical range of cursor, limit and skip of type u64,
    // therwise a database error will be returned when querying
    fn from(page: PaginationRequest) -> Self {
        PageRequest {
            cursor: page
                .cursor
                .map(|cursor| cursor.min(i64::MAX as u64))
                .unwrap_or(if page.order.is_asc() {
                    0u64
                } else {
                    i64::MAX as u64
                }),
            count: page
                .limit
                .map(|limit| limit.min(i64::MAX as u64))
                .unwrap_or(i64::MAX as u64),
            skip: page
                .skip
                .map(|skip| skip.min(i64::MAX as u64))
                .unwrap_or(0u64),
            is_asc: page.order.is_asc(),
            search_count: page.return_count,
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

    fn is_search_count(&self) -> bool {
        self.search_count
    }

    fn get_total(&self) -> u64 {
        self.is_asc.into()
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
