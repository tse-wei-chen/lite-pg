use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    pub name: String,
    pub version: String,
    pub schema: String,
    pub description: Option<String>,
    pub relocatable: bool,
}

pub async fn fetch_extensions(pool: &PgPool) -> Result<Vec<ExtensionInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            e.extname AS name,
            e.extversion AS version,
            n.nspname AS schema,
            c.description,
            e.extrelocatable AS relocatable
        FROM pg_catalog.pg_extension e
        JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
        LEFT JOIN pg_catalog.pg_description c
            ON c.objoid = e.oid AND c.classoid = 'pg_catalog.pg_extension'::regclass
        ORDER BY e.extname
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ExtensionInfo {
            name: r.get("name"),
            version: r.get("version"),
            schema: r.get("schema"),
            description: r.get("description"),
            relocatable: r.get("relocatable"),
        })
        .collect())
}


