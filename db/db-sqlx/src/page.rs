use super::*;

pub(crate) const COUNT_COLUMN: &str = "count";

pub fn build_query_page_sql(
    mut query: &mut SqlBuilder,
    pagination: &PaginationRequest,
) -> Result<(String, String)> {
    let sql_sub_query = query.subquery()?;

    if let Some(id) = pagination.cursor {
        let id = i64::try_from(id).unwrap_or(i64::MAX);
        match pagination.order {
            Order::Asc => query = query.and_where_gt("id", id),
            Order::Desc => query = query.and_where_lt("id", id),
        }
    }
    match pagination.order {
        Order::Asc => query = query.order_by("id", false),
        Order::Desc => query = query.order_by("id", true),
    }
    query = query.limit(pagination.limit.unwrap_or(u16::MAX));

    let query = query.sql()?.trim_end_matches(';').to_string();
    let sub_query_for_count = fetch_count_sql(&format!("{} res", sql_sub_query));

    Ok((query, sub_query_for_count))
}

pub fn build_next_cursor(
    limit: u16,
    last_id: u64,
    records_size: usize,
    total: Option<u64>,
) -> Option<u64> {
    let mut next_cursor = None;
    if records_size == limit as usize {
        if let Some(total) = total {
            if total > limit as u64 {
                next_cursor = Some(last_id)
            }
        } else {
            next_cursor = Some(last_id);
        }
    }
    next_cursor
}
