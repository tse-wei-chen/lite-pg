use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    pub owner: String,
    pub encoding: String,
    pub collate: String,
    pub ctype: String,
    pub is_template: bool,
    pub connection_limit: i32,
    pub size: String,
    pub tablespace: String,
}

pub async fn fetch_databases(pool: &PgPool) -> Result<Vec<DatabaseInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            d.datname,
            COALESCE(u.usename, '') AS owner,
            pg_encoding_to_char(d.encoding) AS encoding,
            d.datcollate AS collate,
            d.datctype AS ctype,
            d.datistemplate AS is_template,
            d.datconnlimit AS connection_limit,
            pg_database_size(d.datname) AS size,
            COALESCE(spc.spcname, 'pg_default') AS tablespace
        FROM pg_catalog.pg_database d
        LEFT JOIN pg_catalog.pg_user u ON d.datdba = u.usesysid
        LEFT JOIN pg_catalog.pg_tablespace spc ON d.dattablespace = spc.oid
        ORDER BY d.datname
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let size_bytes: i64 = r.get("size");
            DatabaseInfo {
                name: r.get("datname"),
                owner: r.get("owner"),
                encoding: r.get("encoding"),
                collate: r.get("collate"),
                ctype: r.get("ctype"),
                is_template: r.get("is_template"),
                connection_limit: r.get("connection_limit"),
                size: format_size(size_bytes),
                tablespace: r.get("tablespace"),
            }
        })
        .collect())
}

pub fn generate_create_database_sql(name: &str, owner: &str, encoding: &str) -> String {
    let mut sql = format!("CREATE DATABASE {}", crate::util::quote_ident(name));
    if !owner.is_empty() {
        sql.push_str(&format!(" OWNER {}", crate::util::quote_ident(owner)));
    }
    if !encoding.is_empty() {
        sql.push_str(&format!(" ENCODING {}", crate::util::quote_literal(encoding)));
    }
    sql.push(';');
    sql
}

pub fn generate_drop_database_sql(name: &str) -> String {
    format!("DROP DATABASE IF EXISTS {};", crate::util::quote_ident(name))
}

fn format_size(bytes: i64) -> String {
    if bytes > 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes > 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes > 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
