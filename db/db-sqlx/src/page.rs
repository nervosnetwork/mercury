use super::*;

pub(crate) const COUNT_COLUMN: &str = "count";

pub fn build_query_page_sql(
    mut query: &mut SqlBuilder,
    pagination: &PaginationRequest,
) -> Result<(String, String)> {
    if let Some(id) = pagination.cursor {
        let id = i64::try_from(id).unwrap_or(i64::MAX);
        match pagination.order {
            Order::Asc => query = query.and_where_ge("id", id),
            Order::Desc => query = query.and_where_le("id", id),
        }
    }
    let sql_sub_query = query.subquery()?;
    match pagination.order {
        Order::Asc => query = query.order_by("id", false),
        Order::Desc => query = query.order_by("id", true),
    }
    if let Some(limit) = pagination.limit {
        let limit = i64::try_from(limit)?;
        query = query.limit(limit);
    }

    let query = query.sql()?.trim_end_matches(';').to_string();
    let sub_query_for_count = fetch_count_sql(&format!("{} res", sql_sub_query));

    Ok((query, sub_query_for_count))
}

pub(crate) fn generate_next_cursor(
    limit: u16,
    records: &[AnyRow],
    total: Option<u64>,
) -> Option<u64> {
    let mut next_cursor = None;
    if records.len() == limit as usize {
        let last = records.last().unwrap().get::<i64, _>("id") as u64;
        if let Some(total) = total {
            if total > limit as u64 {
                next_cursor = Some(last)
            }
        } else {
            next_cursor = Some(last);
        }
    }
    next_cursor
}

pub(crate) fn fetch_count_sql(table_name: &str) -> String {
    format!("SELECT COUNT(*) as {} FROM {}", COUNT_COLUMN, table_name)
}
