use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct ServerOverview {
    pub version: String,
    pub uptime: String,
    pub active_connections: i64,
    pub max_connections: i32,
    pub total_db_size: String,
    pub num_databases: i64,
    pub server_time: String,
}

#[derive(Debug, Clone)]
pub struct DbStatEntry {
    pub name: String,
    pub size: String,
    pub connections: i64,
    pub transactions: i64,
    pub cache_hit_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct ActiveQuery {
    pub pid: i32,
    pub user: String,
    pub database: String,
    pub state: String,
    pub query: String,
    pub duration: String,
    pub wait_event: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TableStatEntry {
    pub schema: String,
    pub table: String,
    pub seq_scan: i64,
    pub idx_scan: i64,
    pub n_tup_ins: i64,
    pub n_tup_upd: i64,
    pub n_tup_del: i64,
    pub n_dead_tup: i64,
}

pub async fn fetch_server_overview(pool: &PgPool) -> Result<ServerOverview> {
    let row = sqlx::query(
        r#"
        SELECT
            version() as version,
            pg_postmaster_start_time() as start_time,
            now() as server_time,
            (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') as active_conns,
            (SELECT current_setting('max_connections')::int) as max_conns,
            (SELECT count(*) FROM pg_database) as num_dbs
        "#,
    )
    .fetch_one(pool)
    .await?;

    let version: String = row.get("version");
    let start_time: chrono::DateTime<chrono::Utc> = row.get("start_time");
    let server_time: chrono::DateTime<chrono::Utc> = row.get("server_time");
    let uptime = server_time - start_time;
    let uptime_str = format!(
        "{}d {}h {}m {}s",
        uptime.num_days(),
        uptime.num_hours() % 24,
        uptime.num_minutes() % 60,
        uptime.num_seconds() % 60,
    );

    let total_size_row = sqlx::query(
        r#"SELECT sum(pg_database_size(datname))::int8 as total_size FROM pg_database"#,
    )
    .fetch_one(pool)
    .await?;
    let total_size: i64 = total_size_row.get("total_size");

    Ok(ServerOverview {
        version: version.lines().next().unwrap_or(&version).to_string(),
        uptime: uptime_str,
        active_connections: row.get("active_conns"),
        max_connections: row.get("max_conns"),
        total_db_size: format_size(total_size),
        num_databases: row.get("num_dbs"),
        server_time: server_time.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    })
}

pub async fn fetch_db_stats(pool: &PgPool) -> Result<Vec<DbStatEntry>> {
    let rows = sqlx::query(
        r#"
        SELECT
            s.datname,
            pg_database_size(s.datname) as size,
            s.numbackends as connections,
            s.xact_commit + s.xact_rollback as transactions,
            CAST(
                CASE WHEN s.blks_hit + s.blks_read > 0
                    THEN round((s.blks_hit::numeric / (s.blks_hit + s.blks_read) * 100), 1)
                    ELSE 0
                END AS double precision
            ) as cache_hit_ratio
        FROM pg_stat_database s
        JOIN pg_database d ON s.datname = d.datname
        WHERE d.datistemplate = false
        ORDER BY s.datname
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let connections_i32: i32 = r.get("connections");
            let transactions_i64: i64 = r.get("transactions");

            DbStatEntry {
                name: r.get("datname"),
                size: format_size(r.get::<i64, _>("size")),
                connections: connections_i32.into(),
                transactions: transactions_i64,
                cache_hit_ratio: r.get::<f64, _>("cache_hit_ratio"),
            }
        })
        .collect())
}

pub async fn fetch_active_queries(pool: &PgPool) -> Result<Vec<ActiveQuery>> {
    let rows = sqlx::query(
        r#"
        SELECT
            pid,
            usename as user,
            datname as database,
            state,
            query,
            wait_event,
            CASE WHEN state = 'active' AND query_start IS NOT NULL
                THEN extract(epoch from now() - query_start)::float8
                ELSE 0
            END as duration
        FROM pg_stat_activity
        WHERE state != 'idle'
          AND pid != pg_backend_pid()
        ORDER BY query_start DESC NULLS LAST
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let dur_secs: f64 = r.get("duration");
            let duration = if dur_secs > 3600.0 {
                format!("{:.1}h", dur_secs / 3600.0)
            } else if dur_secs > 60.0 {
                format!("{:.1}m", dur_secs / 60.0)
            } else {
                format!("{:.1}s", dur_secs)
            };
            ActiveQuery {
                pid: r.get("pid"),
                user: r.get("user"),
                database: r.get("database"),
                state: r.get("state"),
                query: r.get("query"),
                duration,
                wait_event: r.get("wait_event"),
            }
        })
        .collect())
}

pub async fn fetch_table_stats(pool: &PgPool, schema: Option<&str>) -> Result<Vec<TableStatEntry>> {
    let rows = if let Some(s) = schema {
        sqlx::query(
            r#"
            SELECT
                schemaname as schema,
                relname as table,
                seq_scan,
                idx_scan,
                n_tup_ins,
                n_tup_upd,
                n_tup_del,
                n_dead_tup
            FROM pg_stat_user_tables
            WHERE schemaname = $1
            ORDER BY schemaname, relname
            "#,
        )
        .bind(s)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT
                schemaname as schema,
                relname as table,
                seq_scan,
                idx_scan,
                n_tup_ins,
                n_tup_upd,
                n_tup_del,
                n_dead_tup
            FROM pg_stat_user_tables
            ORDER BY schemaname, relname
            "#,
        )
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| TableStatEntry {
            schema: r.get("schema"),
            table: r.get("table"),
            seq_scan: r.get("seq_scan"),
            idx_scan: r.get("idx_scan"),
            n_tup_ins: r.get("n_tup_ins"),
            n_tup_upd: r.get("n_tup_upd"),
            n_tup_del: r.get("n_tup_del"),
            n_dead_tup: r.get("n_dead_tup"),
        })
        .collect())
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
