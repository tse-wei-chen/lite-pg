use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(
    host: &str,
    port: u16,
    user: &str,
    password: Option<&str>,
    dbname: &str,
) -> Result<PgPool> {
    let conn_str = match password {
        Some(pwd) => format!(
            "postgres://{}:{}@{}:{}/{}",
            user, pwd, host, port, dbname
        ),
        None => format!("postgres://{}@{}:{}/{}", user, host, port, dbname),
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&conn_str)
        .await?;

    Ok(pool)
}
