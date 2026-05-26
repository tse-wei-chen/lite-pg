use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct RoleInfo {
    pub name: String,
    pub superuser: bool,
    pub login: bool,
    pub create_db: bool,
    pub create_role: bool,
    pub replication: bool,
    pub bypass_rls: bool,
    pub conn_limit: i32,
    pub valid_until: Option<String>,
    pub member_of: Vec<String>,
    pub use_count: i64,
}

pub async fn fetch_roles(pool: &PgPool) -> Result<Vec<RoleInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            r.rolname AS name,
            r.rolsuper AS superuser,
            r.rolcanlogin AS can_login,
            r.rolcreatedb AS create_db,
            r.rolcreaterole AS create_role,
            r.rolreplication AS replication,
            r.rolbypassrls AS bypass_rls,
            r.rolconnlimit AS conn_limit,
            r.rolvaliduntil::text AS valid_until,
            COALESCE(array_length(r.rolmembers, 1), 0)::int8 AS member_count
        FROM pg_catalog.pg_roles r
        ORDER BY r.rolname
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut roles = Vec::new();
    for row in rows {
        let name: String = row.get("name");

        let member_rows = sqlx::query(
            r#"
            SELECT oid::regrole::text AS member_of
            FROM pg_catalog.pg_auth_members m
            JOIN pg_catalog.pg_roles r ON r.oid = m.roleid
            WHERE m.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = $1)
            "#,
        )
        .bind(&name)
        .fetch_all(pool)
        .await?;

        let member_of: Vec<String> = member_rows.iter().map(|r| r.get("member_of")).collect();

        let use_count: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)::int8 FROM pg_stat_activity WHERE usename = $1
            "#,
        )
        .bind(&name)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        roles.push(RoleInfo {
            name,
            superuser: row.get("superuser"),
            login: row.get("can_login"),
            create_db: row.get("create_db"),
            create_role: row.get("create_role"),
            replication: row.get("replication"),
            bypass_rls: row.get("bypass_rls"),
            conn_limit: row.get("conn_limit"),
            valid_until: row.get("valid_until"),
            member_of,
            use_count,
        });
    }

    Ok(roles)
}

pub fn generate_create_role_sql(
    name: &str,
    login: bool,
    superuser: bool,
    create_db: bool,
    create_role: bool,
    replication: bool,
    password: &str,
    conn_limit: i32,
) -> String {
    let mut sql = format!("CREATE ROLE {}", crate::util::quote_ident(name));
    if login {
        sql.push_str(" LOGIN");
    }
    if superuser {
        sql.push_str(" SUPERUSER");
    }
    if create_db {
        sql.push_str(" CREATEDB");
    }
    if create_role {
        sql.push_str(" CREATEROLE");
    }
    if replication {
        sql.push_str(" REPLICATION");
    }
    if !password.is_empty() {
        sql.push_str(&format!(" PASSWORD {}", crate::util::quote_literal(password)));
    }
    sql.push_str(&format!(" CONNECTION LIMIT {}", conn_limit));
    sql.push(';');
    sql
}

pub fn generate_drop_role_sql(name: &str) -> String {
    format!("DROP ROLE IF EXISTS {};", crate::util::quote_ident(name))
}
