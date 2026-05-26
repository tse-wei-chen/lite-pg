use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct PublicationInfo {
    pub name: String,
    pub owner: String,
    pub all_tables: bool,
    pub publish_insert: bool,
    pub publish_update: bool,
    pub publish_delete: bool,
    pub publish_truncate: bool,
    pub tables: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub name: String,
    pub owner: String,
    pub conninfo: String,
    pub publication: String,
    pub enabled: bool,
    pub slot_name: Option<String>,
}

pub async fn fetch_publications(pool: &PgPool) -> Result<Vec<PublicationInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            p.pubname AS name,
            COALESCE(u.usename, '') AS owner,
            p.puballtables AS all_tables,
            p.pubinsert AS publish_insert,
            p.pubupdate AS publish_update,
            p.pubdelete AS publish_delete,
            p.pubtruncate AS publish_truncate
        FROM pg_catalog.pg_publication p
        LEFT JOIN pg_catalog.pg_user u ON p.pubowner = u.usesysid
        ORDER BY p.pubname
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut pubs = Vec::new();
    for row in rows {
        let name: String = row.get("name");

        let table_rows = sqlx::query(
            r#"
            SELECT schemaname || '.' || tablename AS full_name
            FROM pg_catalog.pg_publication_tables
            WHERE pubname = $1
            ORDER BY full_name
            "#,
        )
        .bind(&name)
        .fetch_all(pool)
        .await?;

        let tables: Vec<String> = table_rows.iter().map(|r| r.get("full_name")).collect();

        pubs.push(PublicationInfo {
            name,
            owner: row.get("owner"),
            all_tables: row.get("all_tables"),
            publish_insert: row.get("publish_insert"),
            publish_update: row.get("publish_update"),
            publish_delete: row.get("publish_delete"),
            publish_truncate: row.get("publish_truncate"),
            tables,
        });
    }

    Ok(pubs)
}

pub async fn fetch_subscriptions(pool: &PgPool) -> Result<Vec<SubscriptionInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            s.subname AS name,
            COALESCE(u.usename, '') AS owner,
            s.subconninfo AS conninfo,
            s.subpublications::text AS publication,
            s.subenabled AS enabled,
            s.subslotname AS slot_name
        FROM pg_catalog.pg_subscription s
        LEFT JOIN pg_catalog.pg_user u ON s.subowner = u.usesysid
        ORDER BY s.subname
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let pub_str: String = r.get("publication");
            let pub_clean = pub_str
                .trim_matches('{')
                .trim_matches('}')
                .to_string();
            SubscriptionInfo {
                name: r.get("name"),
                owner: r.get("owner"),
                conninfo: r.get("conninfo"),
                publication: pub_clean,
                enabled: r.get("enabled"),
                slot_name: r.get("slot_name"),
            }
        })
        .collect())
}


