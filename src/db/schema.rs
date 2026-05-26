#![allow(dead_code)]
use crate::util::quote_ident;
use anyhow::Result;
use sqlx::{PgPool, Row};

// ─── Schema tree data structures ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbObjectType {
    Table,
    View,
    MaterializedView,
    Sequence,
    ForeignTable,
}

impl DbObjectType {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Table => "T",
            Self::View => "V",
            Self::MaterializedView => "M",
            Self::Sequence => "S",
            Self::ForeignTable => "F",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Table => "TABLE",
            Self::View => "VIEW",
            Self::MaterializedView => "MATERIALIZED VIEW",
            Self::Sequence => "SEQUENCE",
            Self::ForeignTable => "FOREIGN TABLE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColumnDetail {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub ordinal_position: i32,
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: String,
    pub columns: Vec<String>,
    pub definition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TriggerInfo {
    pub name: String,
    pub event: String,
    pub timing: String,
    pub level: String,
    pub definition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: String,
    pub columns: Vec<String>,
    pub definition: Option<String>,
    pub referenced_schema: Option<String>,
    pub referenced_table: Option<String>,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DbObject {
    pub name: String,
    pub obj_type: DbObjectType,
    pub schema_name: String,
    pub owner: Option<String>,
    pub owned_table: Option<String>,
    pub columns: Vec<ColumnDetail>,
    pub indexes: Vec<IndexInfo>,
    pub triggers: Vec<TriggerInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub row_count: Option<i64>,
    pub size_bytes: Option<i64>,
    pub description: Option<String>,
    pub create_sql: Option<String>,
    pub detail_loaded: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub name: String,
    pub owner: Option<String>,
    pub objects: Vec<DbObject>,
}

// ─── Legacy type aliases for backward compat (removed) ─────────────────────

// ─── Schema query functions ─────────────────────────────────────────────────

/// Get all non-system schemas with their objects (names + types only).
pub async fn fetch_schemas(pool: &PgPool) -> Result<Vec<SchemaInfo>> {
    let schema_rows = sqlx::query(
        r#"
        SELECT
            n.nspname AS schema_name,
            COALESCE(u.usename, '') AS owner
        FROM pg_catalog.pg_namespace n
        LEFT JOIN pg_catalog.pg_user u ON n.nspowner = u.usesysid
        WHERE n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
        ORDER BY n.nspname
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut schemas = Vec::new();
    for srow in schema_rows {
        let schema_name: String = srow.get("schema_name");
        let owner: String = srow.get("owner");

        let obj_rows = sqlx::query(
            r#"
            SELECT
                c.relname AS name,
                CASE c.relkind
                    WHEN 'r' THEN 'TABLE'
                    WHEN 'v' THEN 'VIEW'
                    WHEN 'm' THEN 'MATERIALIZED VIEW'
                    WHEN 'S' THEN 'SEQUENCE'
                    WHEN 'f' THEN 'FOREIGN TABLE'
                END AS object_type,
                ot.relname AS owned_table
            FROM pg_catalog.pg_class c
            LEFT JOIN pg_catalog.pg_depend d
                ON d.objid = c.oid AND d.deptype = 'a' AND c.relkind = 'S'
            LEFT JOIN pg_catalog.pg_class ot
                ON ot.oid = d.refobjid AND ot.relkind IN ('r', 'v')
            WHERE c.relnamespace = (
                SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = $1
            )
              AND c.relkind IN ('r', 'v', 'm', 'S', 'f')
              AND c.relpersistence = 'p'
            ORDER BY c.relname
            "#,
        )
        .bind(&schema_name)
        .fetch_all(pool)
        .await?;

        let objects: Vec<DbObject> = obj_rows
            .iter()
            .map(|r| {
                let type_str: String = r.get("object_type");
                let obj_type = match type_str.as_str() {
                    "TABLE" => DbObjectType::Table,
                    "VIEW" => DbObjectType::View,
                    "MATERIALIZED VIEW" => DbObjectType::MaterializedView,
                    "SEQUENCE" => DbObjectType::Sequence,
                    "FOREIGN TABLE" => DbObjectType::ForeignTable,
                    _ => DbObjectType::Table,
                };
                let owned_table: Option<String> = r.try_get("owned_table").ok().flatten();
                DbObject {
                    name: r.get("name"),
                    obj_type,
                    schema_name: schema_name.clone(),
                    owner: None,
                    owned_table,
                    columns: Vec::new(),
                    indexes: Vec::new(),
                    triggers: Vec::new(),
                    constraints: Vec::new(),
                    row_count: None,
                    size_bytes: None,
                    description: None,
                    create_sql: None,
                    detail_loaded: false,
                }
            })
            .collect();

        schemas.push(SchemaInfo {
            name: schema_name,
            owner: if owner.is_empty() { None } else { Some(owner) },
            objects,
        });
    }

    Ok(schemas)
}

/// Fetch full detail for one object (columns, indexes, triggers, constraints).
pub async fn fetch_object_detail(
    pool: &PgPool,
    schema: &str,
    object: &str,
    obj_type: &DbObjectType,
) -> Result<DbObject> {
    let columns = fetch_columns(pool, schema, object).await?;
    let indexes = fetch_indexes(pool, schema, object).await?;
    let triggers = fetch_triggers(pool, schema, object).await?;
    let constraints = fetch_constraints(pool, schema, object).await?;
    let row_count = if *obj_type == DbObjectType::Table {
        fetch_row_count(pool, schema, object).await.ok().flatten()
    } else {
        None
    };
    let size = fetch_size(pool, schema, object).await.ok().flatten();
    let description = fetch_comment(pool, schema, object).await.ok().flatten();
    let owner = fetch_owner(pool, schema, object).await.ok().flatten();
    let create_sql = if *obj_type == DbObjectType::Table {
        None // could be generated from columns + constraints
    } else {
        None
    };

    let is_primary_cols: Vec<String> = constraints
        .iter()
        .filter(|c| c.constraint_type == "PRIMARY KEY")
        .flat_map(|c| c.columns.clone())
        .collect();

    let columns: Vec<ColumnDetail> = columns
        .into_iter()
        .map(|mut col| {
            col.is_primary = is_primary_cols.contains(&col.name);
            col
        })
        .collect();

    Ok(DbObject {
        name: object.to_string(),
        obj_type: obj_type.clone(),
        schema_name: schema.to_string(),
        owner,
        owned_table: None,
        columns,
        indexes,
        triggers,
        constraints,
        row_count,
        size_bytes: size,
        description,
        create_sql,
        detail_loaded: true,
    })
}

/// SELECT * FROM "schema"."table" with LIMIT / OFFSET.
pub async fn fetch_table_data(
    pool: &PgPool,
    schema: &str,
    table: &str,
    limit: u64,
    offset: u64,
) -> crate::db::QueryResult {
    let sql = format!(
        r#"SELECT * FROM "{}"."{}" LIMIT {} OFFSET {}"#,
        schema, table, limit, offset
    );
    crate::db::execute_query(pool, &sql).await
}

/// Generate DDL for a database object.
pub async fn fetch_ddl(
    pool: &PgPool,
    schema: &str,
    object: &str,
    obj_type: &DbObjectType,
) -> Result<String> {
    match obj_type {
        DbObjectType::View | DbObjectType::MaterializedView => {
            let mat = matches!(obj_type, DbObjectType::MaterializedView);
            let sql = format!(
                r#"SELECT pg_get_viewdef({}, true) AS ddl"#,
                crate::util::build_regclass(schema, object)
            );
            let row = sqlx::query(&sql).fetch_one(pool).await?;
            let def: String = row.get("ddl");
            let kind = if mat { "MATERIALIZED VIEW" } else { "VIEW" };
            Ok(format!(
                "CREATE {} \"{}\".\"{}\" AS\n{};",
                kind, schema, object, def
            ))
        }
        DbObjectType::Table => {
            build_create_table(pool, schema, object).await
        }
        DbObjectType::Sequence => {
            Ok(format!(
                "-- SEQUENCE \"{}\".\"{}\"\n-- (use \\d+ in psql for full definition)",
                schema, object
            ))
        }
        DbObjectType::ForeignTable => {
            Ok(format!(
                "-- FOREIGN TABLE \"{}\".\"{}\"\n-- Definition depends on foreign server",
                schema, object
            ))
        }
    }
}

async fn build_create_table(pool: &PgPool, schema: &str, table: &str) -> Result<String> {
    let columns = fetch_columns(pool, schema, table).await?;
    let indexes = fetch_indexes(pool, schema, table).await?;
    let constraints = fetch_constraints(pool, schema, table).await?;
    let comment = fetch_comment(pool, schema, table).await.ok().flatten();

    let mut ddl = String::new();
    ddl.push_str(&format!("CREATE TABLE {}.{} (\n", quote_ident(schema), quote_ident(table)));

    let mut col_lines = Vec::new();
    for col in &columns {
        let mut line = format!("    {}", quote_ident(&col.name));
        line.push_str(&format!(" {}", col.data_type));
        if !col.is_nullable {
            line.push_str(" NOT NULL");
        }
        if let Some(ref def) = col.default_value {
            line.push_str(&format!(" DEFAULT {}", def));
        }
        col_lines.push(line);
    }

    // Add constraints inline
    for con in &constraints {
        let mut line = format!("    CONSTRAINT {} {}", con.name, con.constraint_type);
        if !con.columns.is_empty() {
            line.push_str(&format!(
                " ({})",
                con.columns
                    .iter()
                    .map(|c| quote_ident(c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let (Some(ref rt), Some(ref rs)) = (&con.referenced_table, &con.referenced_schema) {
            line.push_str(&format!(" REFERENCES {}.{}", quote_ident(rs), quote_ident(rt)));
            if !con.referenced_columns.is_empty() {
                line.push_str(&format!(
                    " ({})",
                    con.referenced_columns
                        .iter()
                        .map(|c| quote_ident(c))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(ref od) = con.on_delete {
                let action = match od.as_str() {
                    "a" => "NO ACTION",
                    "r" => "RESTRICT",
                    "c" => "CASCADE",
                    "n" => "SET NULL",
                    "d" => "SET DEFAULT",
                    _ => "NO ACTION",
                };
                line.push_str(&format!(" ON DELETE {}", action));
            }
        }
        col_lines.push(line);
    }

    ddl.push_str(&col_lines.join(",\n"));
    ddl.push_str("\n);\n");

    // Separate CREATE INDEX statements
    for idx in &indexes {
        if !idx.is_primary {
            if let Some(ref def) = idx.definition {
                ddl.push_str(def);
                ddl.push_str(";\n");
            }
        }
    }

    // COMMENT
    if let Some(ref desc) = comment {
        ddl.push_str(&format!(
            "COMMENT ON TABLE {}.{} IS '{}';\n",
            quote_ident(schema), quote_ident(table), desc
        ));
    }

    Ok(ddl)
}

// ─── Internal helpers ──────────────────────────────────────────────────────

async fn fetch_columns(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<ColumnDetail>> {
    let rows = sqlx::query(
        r#"
        SELECT
            c.column_name,
            c.data_type,
            c.is_nullable,
            c.column_default,
            c.ordinal_position,
            pgd.description
        FROM information_schema.columns c
        LEFT JOIN pg_catalog.pg_description pgd
            ON pgd.objsubid = c.ordinal_position
            AND pgd.objoid = (
                SELECT c2.oid
                FROM pg_catalog.pg_class c2
                JOIN pg_catalog.pg_namespace n ON n.oid = c2.relnamespace
                WHERE n.nspname = $1 AND c2.relname = $2
            )
        WHERE c.table_schema = $1 AND c.table_name = $2
        ORDER BY c.ordinal_position
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let nullable: String = r.get("is_nullable");
            ColumnDetail {
                name: r.get("column_name"),
                data_type: r.get("data_type"),
                is_nullable: nullable == "YES",
                is_primary: false,
                default_value: r.get("column_default"),
                comment: r.get("description"),
                ordinal_position: r.get("ordinal_position"),
            }
        })
        .collect())
}

async fn fetch_indexes(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<IndexInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            i.indexname AS name,
            i.indexdef AS definition,
            am.amname AS index_type,
            idx.indisunique AS is_unique,
            idx.indisprimary AS is_primary
        FROM pg_catalog.pg_indexes i
        JOIN pg_catalog.pg_class c ON c.relname = i.indexname
            AND c.relnamespace = (SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = i.schemaname)
        JOIN pg_catalog.pg_index idx ON idx.indexrelid = c.oid
        JOIN pg_catalog.pg_am am ON am.oid = c.relam
        WHERE i.schemaname = $1 AND i.tablename = $2
        ORDER BY i.indexname
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let mut indexes = Vec::new();
    for r in rows {
        let def: String = r.get("definition");
        let columns = parse_index_columns(&def);
        indexes.push(IndexInfo {
            name: r.get("name"),
            is_unique: r.get("is_unique"),
            is_primary: r.get("is_primary"),
            index_type: r.get("index_type"),
            columns,
            definition: Some(def),
        });
    }
    Ok(indexes)
}

async fn fetch_triggers(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<TriggerInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            tgname AS name,
            pg_get_triggerdef(t.oid) AS definition
        FROM pg_catalog.pg_trigger t
        JOIN pg_catalog.pg_class c ON c.oid = t.tgrelid
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1 AND c.relname = $2 AND t.tgname IS NOT NULL
          AND NOT t.tgisinternal
        ORDER BY tgname
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let def: Option<String> = r.get("definition");
            let (timing, event, level) = parse_trigger_def(&def);
            TriggerInfo {
                name: r.get("name"),
                timing,
                event,
                level,
                definition: def,
            }
        })
        .collect())
}

async fn fetch_constraints(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<ConstraintInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            tc.constraint_name AS name,
            tc.constraint_type,
            pg_get_constraintdef(con.oid) AS definition,
            ccu.table_schema AS ref_schema,
            ccu.table_name AS ref_table,
            ccu.column_name AS ref_column,
            con.confdeltype::text AS on_delete,
            con.confupdtype::text AS on_update
        FROM information_schema.table_constraints tc
        JOIN pg_catalog.pg_constraint con
            ON con.conname = tc.constraint_name
            AND con.connamespace = (SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = tc.table_schema)
        LEFT JOIN information_schema.constraint_column_usage ccu
            ON ccu.constraint_name = tc.constraint_name
            AND ccu.table_schema = tc.table_schema
        WHERE tc.table_schema = $1
          AND tc.table_name = $2
          AND tc.constraint_type IN ('PRIMARY KEY', 'FOREIGN KEY', 'UNIQUE', 'CHECK')
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let mut constraints = Vec::new();
    for r in rows {
        let ctype: String = r.get("constraint_type");
        let cols = fetch_constraint_columns(pool, schema, &r.get::<String, _>("name")).await?;
        constraints.push(ConstraintInfo {
            name: r.get("name"),
            constraint_type: ctype,
            columns: cols,
            definition: r.get("definition"),
            referenced_schema: r.get("ref_schema"),
            referenced_table: r.get("ref_table"),
            referenced_columns: if r.get::<Option<String>, _>("ref_column").is_some() {
                vec![r.get::<String, _>("ref_column")]
            } else {
                Vec::new()
            },
            on_delete: r.get("on_delete"),
            on_update: r.get("on_update"),
        });
    }
    Ok(constraints)
}

async fn fetch_constraint_columns(pool: &PgPool, schema: &str, constraint: &str) -> Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT column_name
        FROM information_schema.constraint_column_usage
        WHERE table_schema = $1 AND constraint_name = $2
        "#,
    )
    .bind(schema)
    .bind(constraint)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| r.get("column_name")).collect())
}

async fn fetch_row_count(pool: &PgPool, schema: &str, table: &str) -> Result<Option<i64>> {
    let sql = format!(
        r#"SELECT reltuples::bigint AS cnt FROM pg_catalog.pg_class WHERE oid = {}"#,
        crate::util::build_regclass(schema, table)
    );
    let row = sqlx::query(&sql).fetch_optional(pool).await?;
    Ok(row.map(|r| r.get("cnt")))
}

async fn fetch_size(pool: &PgPool, schema: &str, table: &str) -> Result<Option<i64>> {
    let sql = format!(
        r#"SELECT pg_total_relation_size({}) AS size"#,
        crate::util::build_regclass(schema, table)
    );
    let row = sqlx::query(&sql).fetch_optional(pool).await?;
    Ok(row.map(|r| r.get("size")))
}

async fn fetch_comment(pool: &PgPool, schema: &str, table: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        r#"
        SELECT pgd.description
        FROM pg_catalog.pg_description pgd
        JOIN pg_catalog.pg_class c ON c.oid = pgd.objoid
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1 AND c.relname = $2 AND pgd.objsubid = 0
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| r.get::<Option<String>, _>("description")))
}

async fn fetch_owner(pool: &PgPool, schema: &str, object: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        r#"
        SELECT COALESCE(u.usename, '') AS owner
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN pg_catalog.pg_user u ON c.relowner = u.usesysid
        WHERE n.nspname = $1 AND c.relname = $2
        "#,
    )
    .bind(schema)
    .bind(object)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| {
        let owner: String = r.get("owner");
        if owner.is_empty() { None } else { Some(owner) }
    }))
}

/// Batch-fetch columns+indexes+triggers+constraints for many objects.
/// Runs 4 queries in parallel, each using a CTE for all objects.
pub async fn prefetch_all_details(
    pool: &PgPool,
    objects: &[(String, String, crate::db::DbObjectType)],
) -> Result<
    Vec<(
        String,                         // schema
        String,                         // name
        Vec<ColumnDetail>,
        Vec<IndexInfo>,
        Vec<TriggerInfo>,
        Vec<ConstraintInfo>,
        Option<i64>,                    // row_count
        Option<i64>,                    // size_bytes
    )>,
> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }

    let in_clause = build_in_clause(objects);

    let col_sql = format!(
        "WITH o(s,n) AS (VALUES {in_clause})
         SELECT o.s, o.n,
                c.column_name, c.data_type, c.is_nullable, c.column_default,
                c.ordinal_position, pgd.description,
                cl.reltuples::bigint AS row_count,
                pg_total_relation_size((quote_ident(o.s)||'.'||quote_ident(o.n))::text) AS total_size
         FROM o
         LEFT JOIN information_schema.columns c ON c.table_schema=o.s AND c.table_name=o.n
         LEFT JOIN pg_catalog.pg_description pgd ON pgd.objsubid=c.ordinal_position
             AND pgd.objoid=(SELECT c2.oid FROM pg_catalog.pg_class c2
                             JOIN pg_catalog.pg_namespace n ON n.oid=c2.relnamespace
                             WHERE n.nspname=o.s AND c2.relname=o.n)
         LEFT JOIN pg_catalog.pg_class cl ON cl.relname=o.n
             AND cl.relnamespace=(SELECT n.oid FROM pg_catalog.pg_namespace n WHERE n.nspname=o.s)
         ORDER BY o.s, o.n, c.ordinal_position"
    );
    let col_fut = sqlx::query(&col_sql).fetch_all(pool);

    let idx_sql = format!(
        "WITH o(s,n) AS (VALUES {in_clause})
         SELECT o.s, o.n,
                i.indexname AS name, i.indexdef AS definition,
                am.amname AS index_type, idx.indisunique AS is_unique,
                idx.indisprimary AS is_primary
         FROM o
         JOIN pg_catalog.pg_indexes i ON i.schemaname=o.s AND i.tablename=o.n
         JOIN pg_catalog.pg_class c ON c.relname=i.indexname
         JOIN pg_catalog.pg_index idx ON idx.indexrelid=c.oid
         JOIN pg_catalog.pg_am am ON am.oid=c.relam
         ORDER BY o.s, o.n, i.indexname"
    );
    let idx_fut = sqlx::query(&idx_sql).fetch_all(pool);

    let trg_sql = format!(
        "WITH o(s,n) AS (VALUES {in_clause})
         SELECT o.s, o.n,
                tgname AS name, pg_get_triggerdef(t.oid) AS definition
         FROM o
         JOIN pg_catalog.pg_class c ON c.relname=o.n
             AND c.relnamespace=(SELECT n.oid FROM pg_catalog.pg_namespace n WHERE n.nspname=o.s)
         JOIN pg_catalog.pg_trigger t ON t.tgrelid=c.oid
         WHERE t.tgname IS NOT NULL AND NOT t.tgisinternal
         ORDER BY o.s, o.n, tgname"
    );
    let trg_fut = sqlx::query(&trg_sql).fetch_all(pool);

    let con_sql = format!(
        "WITH o(s,n) AS (VALUES {in_clause})
         SELECT o.s, o.n,
                tc.constraint_name AS name, tc.constraint_type,
                pg_get_constraintdef(con.oid) AS definition,
                ccu.table_schema AS ref_schema, ccu.table_name AS ref_table,
                 ccu.column_name AS ref_column,
                 con.confdeltype::text AS on_delete, con.confupdtype::text AS on_update
          FROM o
          JOIN information_schema.table_constraints tc ON tc.table_schema=o.s AND tc.table_name=o.n
          JOIN pg_catalog.pg_constraint con ON con.conname=tc.constraint_name
          LEFT JOIN information_schema.constraint_column_usage ccu
              ON ccu.constraint_name=tc.constraint_name AND ccu.table_schema=o.s
          WHERE tc.constraint_type IN ('PRIMARY KEY','FOREIGN KEY','UNIQUE','CHECK')
         ORDER BY o.s, o.n, tc.constraint_name"
    );
    let con_fut = sqlx::query(&con_sql).fetch_all(pool);

    let (col_rows, idx_rows, trg_rows, con_rows) = tokio::join!(col_fut, idx_fut, trg_fut, con_fut);

    let col_rows = col_rows?;
    let idx_rows = idx_rows?;
    let trg_rows = trg_rows?;
    let con_rows = con_rows?;

    // Group columns by (schema, table)
    let mut cols_map: std::collections::HashMap<(String, String), Vec<ColumnDetail>> =
        std::collections::HashMap::new();
    let mut rc_map: std::collections::HashMap<(String, String), Option<i64>> =
        std::collections::HashMap::new();
    let mut sz_map: std::collections::HashMap<(String, String), Option<i64>> =
        std::collections::HashMap::new();

    for r in &col_rows {
        let key: (String, String) = (r.get("s"), r.get("n"));
        if let Ok(col_name) = r.try_get::<String, _>("column_name") {
            let nullable: String = r.get("is_nullable");
            cols_map.entry(key.clone()).or_default().push(ColumnDetail {
                name: col_name,
                data_type: r.get("data_type"),
                is_nullable: nullable == "YES",
                is_primary: false,
                default_value: r.get("column_default"),
                comment: r.get("description"),
                ordinal_position: r.get("ordinal_position"),
            });
        }
        rc_map.entry(key.clone()).or_insert(r.try_get("row_count").ok());
        sz_map.entry(key.clone()).or_insert(r.try_get("total_size").ok());
    }

    // Group indexes
    let mut idx_map: std::collections::HashMap<(String, String), Vec<IndexInfo>> =
        std::collections::HashMap::new();
    for r in &idx_rows {
        let key: (String, String) = (r.get("s"), r.get("n"));
        let def: String = r.get("definition");
        let cols = parse_index_columns(&def);
        idx_map.entry(key).or_default().push(IndexInfo {
            name: r.get("name"),
            is_unique: r.get("is_unique"),
            is_primary: r.get("is_primary"),
            index_type: r.get("index_type"),
            columns: cols,
            definition: Some(def),
        });
    }

    // Group triggers
    let mut trg_map: std::collections::HashMap<(String, String), Vec<TriggerInfo>> =
        std::collections::HashMap::new();
    for r in &trg_rows {
        let key: (String, String) = (r.get("s"), r.get("n"));
        let def: Option<String> = r.get("definition");
        let (timing, event, level) = parse_trigger_def(&def);
        trg_map.entry(key).or_default().push(TriggerInfo {
            name: r.get("name"),
            timing,
            event,
            level,
            definition: def,
        });
    }

    // Group constraints
    let mut con_map: std::collections::HashMap<(String, String), Vec<ConstraintInfo>> =
        std::collections::HashMap::new();
    for r in &con_rows {
        let key: (String, String) = (r.get("s"), r.get("n"));
        let ctype: String = r.get("constraint_type");
        // Parse columns from definition rather than extra query
        let def: Option<String> = r.get("definition");
        let cols = parse_constraint_columns(&def);
        con_map.entry(key).or_default().push(ConstraintInfo {
            name: r.get("name"),
            constraint_type: ctype,
            columns: cols,
            definition: def.clone(),
            referenced_schema: r.get("ref_schema"),
            referenced_table: r.get("ref_table"),
            referenced_columns: if r.get::<Option<String>, _>("ref_column").is_some() {
                vec![r.get::<String, _>("ref_column")]
            } else {
                Vec::new()
            },
            on_delete: r.get("on_delete"),
            on_update: r.get("on_update"),
        });
    }

    // Identify primary key columns
    let mut pk_cols: std::collections::HashMap<(String, String), Vec<String>> =
        std::collections::HashMap::new();
    for ((s, n), constraints) in &con_map {
        for c in constraints {
            if c.constraint_type == "PRIMARY KEY" {
                pk_cols.entry((s.clone(), n.clone())).or_default().extend(c.columns.clone());
            }
        }
    }

    // Mark primary key columns
    for ((s, n), cols) in &mut cols_map {
        if let Some(pks) = pk_cols.get(&(s.clone(), n.clone())) {
            for col in cols.iter_mut() {
                if pks.contains(&col.name) {
                    col.is_primary = true;
                }
            }
        }
    }

    Ok(objects
        .iter()
        .map(|(s, t, _)| {
            let key = (s.clone(), t.clone());
            (
                s.clone(),
                t.clone(),
                cols_map.remove(&key).unwrap_or_default(),
                idx_map.remove(&key).unwrap_or_default(),
                trg_map.remove(&key).unwrap_or_default(),
                con_map.remove(&key).unwrap_or_default(),
                rc_map.remove(&key).flatten(),
                sz_map.remove(&key).flatten(),
            )
        })
        .collect())
}

fn build_in_clause(objects: &[(String, String, crate::db::DbObjectType)]) -> String {
    let mut buf = String::new();
    for (i, (s, t, _)) in objects.iter().enumerate() {
        if i > 0 { buf.push_str(", "); }
        buf.push_str(&format!("({},{})", crate::util::quote_literal(s), crate::util::quote_literal(t)));
    }
    buf
}

fn parse_constraint_columns(def: &Option<String>) -> Vec<String> {
    let d = match def {
        Some(s) => s.as_str(),
        None => return Vec::new(),
    };
    if let Some(start) = d.find('(') {
        if let Some(end) = d.rfind(')') {
            return d[start + 1..end]
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect();
        }
    }
    Vec::new()
}

fn parse_index_columns(def: &str) -> Vec<String> {
    if let Some(start) = def.find('(') {
        if let Some(end) = def.rfind(')') {
            let inner = &def[start + 1..end];
            return inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect();
        }
    }
    Vec::new()
}

fn parse_trigger_def(def: &Option<String>) -> (String, String, String) {
    let d = match def {
        Some(s) => s.as_str(),
        None => return ("UNKNOWN".into(), "UNKNOWN".into(), "UNKNOWN".into()),
    };
    let timing = if d.contains("BEFORE") {
        "BEFORE"
    } else if d.contains("AFTER") {
        "AFTER"
    } else if d.contains("INSTEAD OF") {
        "INSTEAD OF"
    } else {
        "UNKNOWN"
    };
    let level = if d.contains("FOR EACH ROW") {
        "ROW"
    } else if d.contains("FOR EACH STATEMENT") {
        "STATEMENT"
    } else {
        "UNKNOWN"
    };
    let event = if d.contains("INSERT") {
        "INSERT"
    } else if d.contains("UPDATE") {
        "UPDATE"
    } else if d.contains("DELETE") {
        "DELETE"
    } else if d.contains("TRUNCATE") {
        "TRUNCATE"
    } else {
        "UNKNOWN"
    };
    (timing.to_string(), event.to_string(), level.to_string())
}
