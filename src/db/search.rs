use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub object_type: String,
    pub schema: String,
    pub name: String,
    pub description: Option<String>,
}

pub async fn search_objects(pool: &PgPool, query: &str) -> Result<Vec<SearchResult>> {
    let pattern = format!("%{}%", query);
    let mut results = Vec::new();

    // Tables
    let rows = sqlx::query(
        r#"
        SELECT 'TABLE' AS object_type, n.nspname AS schema, c.relname AS name,
               pgd.description AS description
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN pg_catalog.pg_description pgd
            ON pgd.objoid = c.oid AND pgd.objsubid = 0
        WHERE c.relkind IN ('r', 'p')
          AND n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND c.relname ILIKE $1
        LIMIT 20
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;
    for r in rows {
        results.push(SearchResult {
            object_type: r.get("object_type"),
            schema: r.get("schema"),
            name: r.get("name"),
            description: r.get("description"),
        });
    }

    // Views
    let rows = sqlx::query(
        r#"
        SELECT 'VIEW' AS object_type, n.nspname AS schema, c.relname AS name,
               pgd.description AS description
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN pg_catalog.pg_description pgd
            ON pgd.objoid = c.oid AND pgd.objsubid = 0
        WHERE c.relkind IN ('v', 'm')
          AND n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND c.relname ILIKE $1
        LIMIT 20
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;
    for r in rows {
        results.push(SearchResult {
            object_type: r.get("object_type"),
            schema: r.get("schema"),
            name: r.get("name"),
            description: r.get("description"),
        });
    }

    // Columns
    let rows = sqlx::query(
        r#"
        SELECT 'COLUMN' AS object_type, c.table_schema AS schema,
               c.table_name || '.' || c.column_name AS name,
               c.data_type AS description
        FROM information_schema.columns c
        WHERE c.table_schema NOT LIKE 'pg_%'
          AND c.table_schema != 'information_schema'
          AND c.column_name ILIKE $1
        LIMIT 20
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;
    for r in rows {
        results.push(SearchResult {
            object_type: r.get("object_type"),
            schema: r.get("schema"),
            name: r.get("name"),
            description: r.get("description"),
        });
    }

    // Functions
    let rows = sqlx::query(
        r#"
        SELECT 'FUNCTION' AS object_type, n.nspname AS schema, p.proname AS name,
               pg_get_function_arguments(p.oid) AS description
        FROM pg_catalog.pg_proc p
        JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND p.proname ILIKE $1
        LIMIT 20
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;
    for r in rows {
        results.push(SearchResult {
            object_type: r.get("object_type"),
            schema: r.get("schema"),
            name: r.get("name"),
            description: r.get("description"),
        });
    }

    results.sort_by(|a, b| a.object_type.cmp(&b.object_type));
    Ok(results)
}
