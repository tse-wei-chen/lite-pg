use sqlx::postgres::PgRow;
use sqlx::Column;
use sqlx::Row;
use sqlx::ValueRef;
use sqlx::PgPool;
use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[allow(dead_code)]
    pub affected: u64,
    pub elapsed: std::time::Duration,
    pub error: Option<String>,
}

fn format_cell(row: &PgRow, i: usize) -> String {

    if let Ok(raw) = row.try_get_raw(i) {
        if raw.is_null() {
            return "NULL".to_string();
        }
    }

    if let Ok(v) = row.try_get::<String, _>(i) {
        return v;
    }
    if let Ok(v) = row.try_get::<i16, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<i32, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<i64, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<f32, _>(i) {
        return format!("{v}");
    }
    if let Ok(v) = row.try_get::<f64, _>(i) {
        return format!("{v}");
    }
    if let Ok(v) = row.try_get::<bool, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<uuid::Uuid, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(i) {
        return v.to_string();
    }
    if let Ok(v) = row.try_get::<Vec<u8>, _>(i) {
        return format!("{:?}", v);
    }

    if let Ok(raw) = row.try_get_raw(i) {
        if let Ok(bytes) = raw.as_bytes() {
            return String::from_utf8_lossy(bytes).to_string();
        }
        return String::new();
    }

    "?".to_string()
}

fn format_row(row: &PgRow) -> Vec<String> {
    (0..row.columns().len()).map(|i| format_cell(row, i)).collect()
}

pub async fn execute_query(pool: &PgPool, sql: &str) -> QueryResult {
    let start = Instant::now();
    let trimmed = sql.trim();

    if trimmed.is_empty() {
        return QueryResult {
            elapsed: start.elapsed(),
            error: Some("Empty query".to_string()),
            ..Default::default()
        };
    }

    match sqlx::query(trimmed).fetch_all(pool).await {
        Ok(rows) => {
            if rows.is_empty() {
                return QueryResult {
                    columns: vec!["(no columns)".to_string()],
                    rows: vec![],
                    affected: 0,
                    elapsed: start.elapsed(),
                    ..Default::default()
                };
            }

            let columns: Vec<String> = rows[0]
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<_>>();

            let data: Vec<Vec<String>> = rows.iter().map(format_row).collect();

            QueryResult {
                columns,
                rows: data,
                affected: rows.len() as u64,
                elapsed: start.elapsed(),
                ..Default::default()
            }
        }
        Err(fetch_err) => {
            match sqlx::query(trimmed).execute(pool).await {
                Ok(result) => QueryResult {
                    columns: vec!["RESULT".to_string()],
                    rows: vec![vec![format!("{} rows affected", result.rows_affected())]],
                    elapsed: start.elapsed(),
                    affected: result.rows_affected(),
                    ..Default::default()
                },
                Err(_) => QueryResult {
                    elapsed: start.elapsed(),
                    error: Some(fetch_err.to_string()),
                    ..Default::default()
                },
            }
        }
    }
}

#[allow(dead_code)]
pub async fn execute_explain(pool: &PgPool, sql: &str) -> QueryResult {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return QueryResult {
            error: Some("Empty query".to_string()),
            ..Default::default()
        };
    }
    let explain_sql = format!("EXPLAIN ANALYZE {}", trimmed);
    execute_query(pool, &explain_sql).await
}
