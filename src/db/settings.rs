use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct SettingInfo {
    pub name: String,
    pub value: String,
    pub unit: Option<String>,
    pub category: String,
    pub description: String,
    pub context: String,
    pub setting_type: String,
    pub reset_value: Option<String>,
}

pub async fn fetch_settings(pool: &PgPool) -> Result<Vec<SettingInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            name,
            setting AS value,
            unit,
            category,
            COALESCE(short_desc, '') AS description,
            context,
            vartype AS setting_type,
            reset_val AS reset_value
        FROM pg_catalog.pg_settings
        ORDER BY category, name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| SettingInfo {
            name: r.get("name"),
            value: r.get("value"),
            unit: r.get("unit"),
            category: r.get("category"),
            description: r.get("description"),
            context: r.get("context"),
            setting_type: r.get("setting_type"),
            reset_value: r.get("reset_value"),
        })
        .collect())
}
