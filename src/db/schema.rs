use anyhow::Result;
use sqlx::Row;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

pub async fn fetch_schema(pool: &PgPool) -> Result<Vec<TableInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
        ORDER BY table_name
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut tables = Vec::new();
    for row in rows {
        let table_name: String = row.get("table_name");
        let columns = fetch_columns(pool, &table_name).await?;
        tables.push(TableInfo {
            name: table_name,
            columns,
        });
    }

    Ok(tables)
}

async fn fetch_columns(pool: &PgPool, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT column_name, data_type, is_nullable
        FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = $1
        ORDER BY ordinal_position
        "#,
    )
    .bind(table_name)
    .fetch_all(pool)
    .await?;

    let columns = rows
        .iter()
        .map(|row| {
            let is_nullable: String = row.get("is_nullable");
            ColumnInfo {
                name: row.get("column_name"),
                data_type: row.get("data_type"),
                is_nullable: is_nullable == "YES",
            }
        })
        .collect();

    Ok(columns)
}
