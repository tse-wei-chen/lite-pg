use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub schema: String,
    pub args: String,
    pub return_type: String,
    pub language: String,
    pub is_aggregate: bool,
    pub is_window: bool,
    pub security_definer: bool,
    pub volatility: String,
    pub parallel: String,
    pub definition: Option<String>,
}

pub async fn fetch_functions(pool: &PgPool, schema: &str) -> Result<Vec<FunctionInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            p.proname AS name,
            n.nspname AS schema,
            pg_get_function_arguments(p.oid) AS args,
            pg_get_function_result(p.oid) AS return_type,
            l.lanname AS language,
            p.prokind = 'a' AS is_aggregate,
            p.prokind = 'w' AS is_window,
            p.prosecdef AS security_definer,
            CASE p.provolatile
                WHEN 'v' THEN 'VOLATILE'
                WHEN 's' THEN 'STABLE'
                WHEN 'i' THEN 'IMMUTABLE'
            END AS volatility,
            CASE p.proparallel
                WHEN 's' THEN 'SAFE'
                WHEN 'r' THEN 'RESTRICTED'
                WHEN 'u' THEN 'UNSAFE'
            END AS parallel
        FROM pg_catalog.pg_proc p
        JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
        JOIN pg_catalog.pg_language l ON l.oid = p.prolang
        WHERE n.nspname = $1
          AND p.prorettype != 0
        ORDER BY p.proname
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let mut functions = Vec::new();
    for row in rows {
        let name: String = row.get("name");
        let schema: String = row.get("schema");

        let def = fetch_function_definition(pool, &schema, &name).await.ok().flatten();

        functions.push(FunctionInfo {
            name,
            schema,
            args: row.get("args"),
            return_type: row.get("return_type"),
            language: row.get("language"),
            is_aggregate: row.get("is_aggregate"),
            is_window: row.get("is_window"),
            security_definer: row.get("security_definer"),
            volatility: row.get("volatility"),
            parallel: row.get("parallel"),
            definition: def,
        });
    }

    Ok(functions)
}

async fn fetch_function_definition(
    pool: &PgPool,
    schema: &str,
    function: &str,
) -> Result<Option<String>> {
    let row = sqlx::query_scalar::<_, String>(
        r#"
        SELECT pg_get_functiondef(p.oid) AS ddl
        FROM pg_catalog.pg_proc p
        JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = $1 AND p.proname = $2
        LIMIT 1
        "#,
    )
    .bind(schema)
    .bind(function)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
