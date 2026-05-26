use crate::connections::SslMode;
use anyhow::Result;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::PgPool;

pub async fn connect(
    host: &str,
    port: u16,
    user: &str,
    password: Option<&str>,
    dbname: &str,
    ssl_mode: &SslMode,
) -> Result<PgPool> {
    let pg_ssl = match ssl_mode {
        SslMode::Disable => PgSslMode::Disable,
        SslMode::Prefer => PgSslMode::Prefer,
        SslMode::Require => PgSslMode::Require,
    };
    let mut opts = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(user)
        .database(dbname)
        .ssl_mode(pg_ssl);

    if let Some(pwd) = password {
        opts = opts.password(pwd);
    }

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    Ok(pool)
}
